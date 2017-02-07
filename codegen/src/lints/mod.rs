mod utils;

use self::utils::*;

use ::{ROUTE_ATTR, ROUTE_INFO_ATTR};

use std::mem::transmute;
use std::collections::HashMap;

use rustc::lint::{Level, LateContext, LintContext, LintPass, LateLintPass, LintArray};
use rustc::hir::{Item, Expr, Crate, Decl, FnDecl, Body, QPath, PatKind};
use rustc::hir::def::Def;
use rustc::hir::def_id::DefId;
use rustc::ty::Ty;
use rustc::hir::intravisit::{FnKind};
use rustc::hir::Ty_::*;
use rustc::hir::Decl_::*;
use rustc::hir::Expr_::*;

use syntax_pos::Span;
use syntax::symbol::Symbol as Name;
use syntax::ast::NodeId;

const STATE_TYPE: &'static [&'static str] = &["rocket", "request", "state", "State"];

// Information about a specific Rocket instance.
#[derive(Debug, Default)]
struct InstanceInfo {
    // Mapping from mounted struct info to the span of the mounted call.
    mounted: HashMap<DefId, Span>,
    // Mapping from managed types to the span of the manage call.
    managed: HashMap<Ty<'static>, Span>,
}

/// A `Receiver` captures the "receiver" of a Rocket instance method call. A
/// Receiver can be an existing instance of Rocket or a call to an Rocket
/// initialization function.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum Receiver {
    Instance(DefId, Span),
    Call(NodeId, Span),
}

impl Receiver {
    /// Returns the span associated with the receiver.
    pub fn span(&self) -> Span {
        match *self {
            Receiver::Instance(_, sp) | Receiver::Call(_, sp) => sp
        }
    }
}

#[derive(Debug, Default)]
pub struct RocketLint {
    // All of the types that were requested as managed state.
    // (fn_name, fn_span, info_struct_def_id, req_type, req_param_span)
    requested: Vec<(Name, Span, DefId, Ty<'static>, Span)>,
    // Mapping from a `Rocket` instance initialization call span (an ignite or
    // custom call) to the collected info about that instance.
    instances: HashMap<Option<Receiver>, InstanceInfo>,
    // Map of all route info structure names found in the program to its defid.
    // This is used to map a declared route to its info structure defid.
    info_structs: HashMap<Name, DefId>,
    // The name, span, and info DefId for all route functions found. The DefId
    // is obtained by indexing into info_structs with the name found in the
    // attribute that Rocket generates.
    declared: Vec<(Name, Span, DefId)>,
    // Mapping from known named Rocket instances to initial receiver. This is
    // used to do a sort-of flow-based analysis. We track variable declarations
    // and link calls to Rocket methods to the (as best as we can tell) initial
    // call to generate that Rocket instance. We use this to group calls to
    // `manage` and `mount` to give more accurate warnings.
    instance_vars: HashMap<DefId, Receiver>,
}

declare_lint!(UNMOUNTED_ROUTE, Warn, "Warn on routes that are unmounted.");

declare_lint!(UNMANAGED_STATE, Warn, "Warn on declared use of unmanaged state.");

impl<'tcx> LintPass for RocketLint {
    fn get_lints(&self) -> LintArray {
        lint_array!(UNMANAGED_STATE, UNMOUNTED_ROUTE)
    }
}

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for RocketLint {
    // This fills out the `instance_vars` table by tracking calls to
    // function/methods that create Rocket instances. If the call is a method
    // call with a receiver that we know is a Rocket instance, then we know it's
    // been moved, and we track that move by linking all definition to the same
    // receiver.
    fn check_decl(&mut self, cx: &LateContext<'a, 'tcx>, decl: &'tcx Decl) {
        // We only care about local declarations...everything else seems very
        // unlikely. This is imperfect, after all.
        if let DeclLocal(ref local) = decl.node {
            // Retrieve the def_id for the new binding.
            let new_def_id = match local.pat.node {
                PatKind::Binding(_, def_id, ..) => def_id ,
                _ => return
            };

            // `init` is the RHS of the declaration.
            if let Some(ref init) = local.init {
                // We only care about declarations that result in Rocket insts.
                if !returns_rocket_instance(cx, init) {
                    return;
                }

                let (expr, span) = match find_initial_receiver(cx, init) {
                    Some(expr) => (expr, expr.span),
                    None => return
                };

                // If the receiver is a path, check if this path was declared
                // before by another binding and use that binding's receiver as
                // this binding's receiver, essentially taking us back in time.
                // If we don't know about it, just insert a new receiver.
                if let ExprPath(QPath::Resolved(_, ref path)) = expr.node {
                    if let Some(old_def_id) = path.def.def_id_opt() {
                        if let Some(&prev) = self.instance_vars.get(&old_def_id) {
                            self.instance_vars.insert(new_def_id, prev);
                        } else {
                            let recvr = Receiver::Instance(old_def_id, span);
                            self.instance_vars.insert(new_def_id, recvr);
                        }
                    }
                }

                // We use a call as a base case. Maybe it's a brand new Rocket
                // instance, maybe it's a function returning a Rocket instance.
                // Who knows. This is where imperfection comes in. We're just
                // going to assume that calls to `mount` and `manage` are
                // grouped with their originating call.
                if let ExprCall(ref expr, ..) = expr.node {
                    let recvr = Receiver::Call(expr.id, span);
                    self.instance_vars.insert(new_def_id, recvr);
                }
            }
        }
    }

    // Here, we collect all of the calls to `manage` and `mount` by instance,
    // where the instance is determined by the receiver of the call. We look up
    // the receiver in the type table we've constructed. If it's there, we use
    // it, if not, we use the call as the receiver.
    fn check_expr(&mut self, cx: &LateContext<'a, 'tcx>, expr: &'tcx Expr) {
        /// Fetches the top-level `Receiver` instance given that a method call
        /// was made to the receiver `rexpr`. Top-level here means "the
        /// original". We search the `instance_vars` table to retrieve it.
        let instance_for = |lint: &mut RocketLint, rexpr: &Expr| -> Option<Receiver> {
            match rexpr.node {
                ExprPath(QPath::Resolved(_, ref p)) => {
                    p.def.def_id_opt()
                        .and_then(|id| lint.instance_vars.get(&id))
                        .map(|recvr| recvr.clone())
                }
                ExprCall(ref c, ..) => Some(Receiver::Call(c.id, rexpr.span)),
                _ => unreachable!()
            }
        };

        if let Some((recvr, args)) = rocket_method_call("manage", cx, expr) {
            let managed_val = &args[0];
            let instance = recvr.and_then(|r| instance_for(self, r));
            if let Some(managed_ty) = cx.tables.expr_ty_opt(managed_val) {
                self.instances.entry(instance)
                    .or_insert_with(|| InstanceInfo::default())
                    .managed
                    .insert(unsafe { transmute(managed_ty) }, managed_val.span);
            }
        }

        if let Some((recvr, args)) = rocket_method_call("mount", cx, expr) {
            let instance = recvr.and_then(|r| instance_for(self, r));
            for def_id in extract_mount_fn_def_ids(cx, &args[1]) {
                self.instances.entry(instance)
                    .or_insert_with(|| InstanceInfo::default())
                    .mounted
                    .insert(def_id, expr.span);
            }
        }

        // This captures a corner case where neither `manage` nor `mount` is
        // called on an instance.
        if let Some((Some(recvr_expr), _)) = rocket_method_call("launch", cx, expr) {
            if let Some(instance) = instance_for(self, recvr_expr) {
                self.instances.entry(Some(instance))
                    .or_insert_with(|| InstanceInfo::default());
            }
        }
    }

    // We collect all of the names and defids for the info structures that
    // Rocket has generated. We do this by simply looking at the attribute,
    // which Rocket's codegen was kind enough to generate.
    fn check_item(&mut self, cx: &LateContext<'a, 'tcx>, item: &'tcx Item) {
        // Return early if this is not a route info structure.
        if !item.attrs.iter().any(|attr| attr.check_name(ROUTE_INFO_ATTR)) {
            return;
        }

        if let Some(def_id) = cx.tcx.hir.opt_local_def_id(item.id) {
            self.info_structs.insert(item.name, def_id);
        }
    }

    /// We do two things here: 1) we find all of the `State` request guards a
    /// user wants, and 2) we find all of the routes declared by the user. We
    /// determine that a function is a route by looking for the attribute that
    /// Rocket declared. We tie the route to the info structure, obtained from
    /// the `check_item` call, so that we can determine if the route was mounted
    /// or not. The tie is done by looking at the name of the info structure in
    /// the attribute that Rocket generated and then looking up the structure in
    /// the `info_structs` map. The structure _must_ be there since Rocket
    /// always generates the structure before the route.
    fn check_fn(&mut self,
                cx: &LateContext<'a, 'tcx>,
                kind: FnKind<'tcx>,
                decl: &'tcx FnDecl,
                _: &'tcx Body,
                fn_sp: Span,
                fn_id: NodeId) {
        // Get the name of the function, if any.
        let fn_name = match kind {
            FnKind::ItemFn(name, ..) => name,
            _ => return
        };

        // Figure out if this is a route function by trying to find the
        // `ROUTE_ATTR` attribute and extracing the info struct's name from it.
        let attr_value = kind.attrs().iter().filter_map(|attr| {
            match attr.check_name(ROUTE_ATTR) {
                false => None,
                true => attr.value.meta_item_list().and_then(|list| list[0].name())
            }
        }).next();

        // Try to get the DEF_ID using the info struct's name from the
        // `info_structs` map. Return early if anything goes awry.
        let def_id = match attr_value {
            Some(val) if self.info_structs.contains_key(&val) => {
                self.info_structs.get(&val).unwrap()
            }
            _ => return
        };

        // Add this to the list of declared routes to check for mounting later
        // unless unmounted routes were explicitly allowed for this function.
        if cx.current_level(UNMOUNTED_ROUTE) != Level::Allow {
            self.declared.push((fn_name, fn_sp, def_id.clone()));
        }

        // If unmanaged state was explicitly allowed for this function, don't
        // record any additional information. Just return now.
        if cx.current_level(UNMANAGED_STATE) == Level::Allow {
            return;
        }

        // Collect all of the `State` types into `tys`.
        let mut tys: Vec<Ty<'static>> = vec![];
        if let Some(sig) = cx.tables.liberated_fn_sigs.get(&fn_id) {
            for input_ty in sig.inputs() {
                let def_id = match input_ty.ty_to_def_id() {
                    Some(id) => id,
                    None => continue
                };

                if match_def_path(cx.tcx, def_id, STATE_TYPE) {
                    if let Some(inner_type) = input_ty.walk_shallow().next() {
                        tys.push(unsafe { transmute(inner_type) });
                    }
                }
            }
        }

        // Collect all of the spans for the `State` parameters.
        let mut spans = vec![];
        for input in decl.inputs.iter() {
            let id = input.id;
            if let TyPath(ref qpath) = input.node {
                if let Def::Struct(defid) = cx.tables.qpath_def(qpath, id) {
                    if match_def_path(cx.tcx, defid, STATE_TYPE) {
                        spans.push(input.span);
                    }
                }
            }
        }

        // Sanity check: we should have as many spans as types.
        if tys.len() != spans.len() {
            panic!("Lint error: unequal ty/span ({}/{})", tys.len(), spans.len());
        }

        // Insert the information we've collected.
        for (ty, span) in tys.into_iter().zip(spans.into_iter()) {
            self.requested.push((fn_name, fn_sp, def_id.clone(), ty, span));
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'a, 'tcx>, _: &'tcx Crate) {
        // Iterate through all the instances, emitting warnings.
        for (instance, info) in self.instances.iter() {
            self.unmounted_warnings(cx, *instance, info);
            self.unmanaged_warnings(cx, *instance, info);
        }
    }
}

impl RocketLint {
    fn unmounted_warnings(&self, cx: &LateContext,
                          rcvr: Option<Receiver>,
                          info: &InstanceInfo) {
        // Emit a warning for all unmounted, declared routes.
        for &(route_name, fn_sp, info_def_id) in self.declared.iter() {
            if !info.mounted.contains_key(&info_def_id) {
                let help_span = rcvr.map(|r| r.span());
                msg_and_help(cx, UNMOUNTED_ROUTE, fn_sp,
                    &format!("the '{}' route is not mounted", route_name),
                    "Rocket will not dispatch requests to unmounted routes.",
                    help_span, "maybe add a call to `mount` here?");
            }
        }
    }

    fn unmanaged_warnings(&self,
                          cx: &LateContext,
                          rcvr: Option<Receiver>,
                          info: &InstanceInfo) {
        for &(_, _, info_def_id, ty, sp) in self.requested.iter() {
            // Don't warn on unmounted routes.
            if !info.mounted.contains_key(&info_def_id) { continue }

            if !info.managed.contains_key(&ty) {
                let help_span = rcvr.map(|r| r.span());
                msg_and_help(cx, UNMANAGED_STATE, sp,
                    &format!("'{}' is not currently being managed by Rocket", ty),
                    "this 'State' request guard will always fail",
                    help_span, "maybe add a call to `manage` here?");
            }
        }
    }
}
