macro_rules! known_content_codings {
    ($cont:ident) => ($cont! {
        Any (is_any): "any content coding", "*",
        // BR (is_br): "Brotli Compressed Data Format", "br",
        // COMPRESS (is_compress): "UNIX \"compress\" data format", "compress",
        // DEFLATE (is_deflate): "\"deflate\" compressed data inside the \"zlib\" data format", "deflate",
        GZIP (is_gzip): "GZIP file format", "gzip",
        IDENTITY (is_identity): "Reserved", "identity",
    })
}
