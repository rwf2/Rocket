use std::borrow::Cow;

type Index = u32;

#[derive(Debug, Clone)]
pub enum IndexedStr {
    Indexed(Index, Index),
    Concrete(Cow<'static, str>)
}

impl IndexedStr {
    /// Whether this string is derived from indexes or not.
    pub fn is_indexed(&self) -> bool {
        match *self {
            IndexedStr::Indexed(..) => true,
            IndexedStr::Concrete(..) => false,
        }
    }

    /// Retrieves the string `self` corresponds to. If `self` is derived from
    /// indexes, the corresponding subslice of `string` is returned. Otherwise,
    /// the concrete string is returned.
    ///
    /// # Panics
    ///
    /// Panics if `self` is an indexed string and `string` is None.
    pub fn to_str<'a>(&'a self, string: Option<&'a str>) -> &'a str {
        if self.is_indexed() && string.is_none() {
            panic!("Cannot convert indexed str to str without base string!")
        }

        match *self {
            IndexedStr::Indexed(i, j) => &string.unwrap()[(i as usize)..(j as usize)],
            IndexedStr::Concrete(ref mstr) => &*mstr,
        }
    }

    pub fn from(needle: &str, haystack: &str) -> Option<IndexedStr> {
        let haystack_start = haystack.as_ptr() as usize;
        let needle_start = needle.as_ptr() as usize;

        if needle_start < haystack_start {
            return None;
        }

        if (needle_start + needle.len()) > (haystack_start + haystack.len()) {
            return None;
        }

        let start = needle_start - haystack_start;
        let end = start + needle.len();
        Some(IndexedStr::Indexed(start as Index, end as Index))
    }
}
