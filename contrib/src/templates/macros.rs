/// Returns a hashset with the extensions of all of the enabled template
/// engines from the set of template engined passed in.
macro_rules! engine_set {
    ($($feature:expr => $engine:ident),+,) => ({
        type RegisterFn = for<'a, 'b> unsafe fn(&'a [(&'b str, &TemplateInfo)]) -> bool;

        let mut set = Vec::new();
        $(
            #[cfg(feature = $feature)]
            fn $engine(set: &mut Vec<(&'static str, RegisterFn)>) {
                set.push(($engine::EXT, $engine::register));
            }

            #[cfg(not(feature = $feature))]
            fn $engine(_: &mut Vec<(&'static str, RegisterFn)>) { }

            $engine(&mut set);
        )+

        set
    });
}

/// Renders the template named `name` with the given template info `info` and
/// context `ctxt` using one of the templates in the template set passed in. It
/// does this by checking if the template's extension matches the engine's
/// extension, and if so, calls the engine's `render` method. All of this only
/// happens for engine's that have been enabled as features by the user.
macro_rules! render_set {
    ($name:expr, $info:expr, $ctxt:expr, $($feature:expr => $engine:ident),+,) => ({
        $(
            #[cfg(feature = $feature)]
            fn $engine<T: Serialize>(name: &str, info: &TemplateInfo, c: &T)
            -> Option<Template> {
                if info.extension == $engine::EXT {
                    let rendered = $engine::render(name, info, c);
                    return Some(Template(rendered, info.data_type.clone()));
                }

                None
            }

            #[cfg(not(feature = $feature))]
            fn $engine<T: Serialize>(_: &str, _: &TemplateInfo, _: &T)
            -> Option<Template> { None }

            if let Some(template) = $engine($name, &$info, $ctxt) {
                return template
            }
         )+
    });
}

