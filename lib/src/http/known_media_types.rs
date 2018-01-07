macro_rules! known_media_types {
    ($cont:ident) => ($cont! {
        Any (is_any): "any media type", "*", "*",
        Binary (is_binary): "binary data", "application", "octet-stream",
        HTML (is_html): "HTML", "text", "html" ; "charset" => "utf-8",
        Plain (is_plain): "plain text", "text", "plain" ; "charset" => "utf-8",
        JSON (is_json): "JSON", "application", "json",
        MsgPack (is_msgpack): "MessagePack", "application", "msgpack",
        Form (is_form): "forms", "application", "x-www-form-urlencoded",
        JavaScript (is_javascript): "JavaScript", "application", "javascript",
        CSS (is_css): "CSS", "text", "css" ; "charset" => "utf-8",
        FormData (is_form_data): "multipart form data", "multipart", "form-data",
        XML (is_xml): "XML", "text", "xml" ; "charset" => "utf-8",
        CSV (is_csv): "CSV", "text", "csv" ; "charset" => "utf-8",
        PNG (is_png): "PNG", "image", "png",
        GIF (is_gif): "GIF", "image", "gif",
        BMP (is_bmp): "BMP", "image", "bmp",
        JPEG (is_jpeg): "JPEG", "image", "jpeg",
        WEBP (is_webp): "WEBP", "image", "webp",
        SVG (is_svg): "SVG", "image", "svg+xml",
        PDF (is_pdf): "PDF", "application", "pdf",
        TTF (is_ttf): "TTF", "application", "font-sfnt",
        OTF (is_otf): "OTF", "application", "font-sfnt",
        WOFF (is_woff): "WOFF", "application", "font-woff",
        WOFF2 (is_woff2): "WOFF2", "font", "woff2",
        JsonApi (is_json_api): "JSON API", "application", "vnd.api+json"
    })
}

macro_rules! known_extensions {
    ($cont:ident) => ($cont! {
        "txt" => Plain,
        "html" => HTML,
        "htm" => HTML,
        "xml" => XML,
        "csv" => CSV,
        "js" => JavaScript,
        "css" => CSS,
        "json" => JSON,
        "png" => PNG,
        "gif" => GIF,
        "bmp" => BMP,
        "jpeg" => JPEG,
        "jpg" => JPEG,
        "webp" => WEBP,
        "svg" => SVG,
        "pdf" => PDF,
        "ttf" => TTF,
        "otf" => OTF,
        "woff" => WOFF,
        "woff2" => WOFF2
    })
}
