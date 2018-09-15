mod uri;

use utils::IdentExt;

use syntax::ast::Path;

pub use self::uri::{uri, uri_internal};

#[inline]
pub fn prefix_path(prefix: &str, path: &mut Path) {
    let last = path.segments.len() - 1;
    let last_seg = &mut path.segments[last];
    last_seg.ident = last_seg.ident.prepend(prefix);
}
