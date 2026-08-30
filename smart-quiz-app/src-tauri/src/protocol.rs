// bankasset:// 自定义协议：bankasset://<bank_id>/<相对路径> → AppData/banks/<bank_id>/assets/
// 统一解析内置/导入题库图片（设计方案 V1.1 §2.2）
use tauri::http::{Request, Response, StatusCode};
use tauri::Manager;

pub fn handle<T: tauri::Runtime>(app: &tauri::AppHandle<T>, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    // WebView2 对非 ASCII 路径（如 B卷-xxx.png）会百分号编码，必须先解码再查文件
    let path = percent_decode(request.uri().path()); // 形如 /<bank_id>/<rel...>
    let parts: Vec<&str> = path.trim_start_matches('/').splitn(2, '/').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return resp(StatusCode::BAD_REQUEST, "text/plain", b"bad request".to_vec());
    }
    // 兼容两种存储形态：img_path 带 "assets/" 前缀（打包脚本产物）或纯相对路径
    let (bank_id, rel) = (parts[0], strip_assets_prefix(parts[1]));
    // 防路径穿越
    if rel.split(['/', '\\']).any(|s| s == "..") || bank_id.contains("..") {
        return resp(StatusCode::FORBIDDEN, "text/plain", b"forbidden".to_vec());
    }
    let Ok(data_dir) = app.path().app_data_dir() else {
        return resp(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", b"no data dir".to_vec());
    };
    let full = data_dir.join("banks").join(bank_id).join("assets").join(rel);
    match std::fs::read(&full) {
        Ok(bytes) => {
            let ct = match full.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
                "png" => "image/png", "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif", "webp" => "image/webp", "bmp" => "image/bmp",
                "svg" => "image/svg+xml", _ => "application/octet-stream",
            };
            resp(StatusCode::OK, ct, bytes)
        }
        Err(_) => resp(StatusCode::NOT_FOUND, "text/plain", b"not found".to_vec()),
    }
}

fn resp(status: StatusCode, ct: &str, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder().status(status)
        .header("Content-Type", ct)
        .header("Access-Control-Allow-Origin", "*")
        .body(body).unwrap()
}

/// img_path 历史上存过 "assets/xxx.png"（打包脚本带前缀），协议层已拼 assets 目录——去掉冗余前缀
fn strip_assets_prefix(rel: &str) -> &str {
    rel.strip_prefix("assets/").unwrap_or(rel)
}

/// 解码 URI 路径中的 %XX 序列（UTF-8），非法序列原样保留
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let hex = |c: u8| -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::{percent_decode, strip_assets_prefix};

    #[test]
    fn strips_redundant_assets_prefix() {
        assert_eq!(strip_assets_prefix("assets/B卷-7b.png"), "B卷-7b.png");
        assert_eq!(strip_assets_prefix("B卷-7b.png"), "B卷-7b.png");
        assert_eq!(strip_assets_prefix("sub/dir.png"), "sub/dir.png");
    }

    #[test]
    fn decodes_percent_encoded_chinese_filename() {
        // "B卷-7b468950.png" 的 UTF-8 百分号编码（WebView2 实际请求形态）
        assert_eq!(percent_decode("/smart-core/assets/B%E5%8D%B7-7b468950.png"),
                   "/smart-core/assets/B卷-7b468950.png");
        // 空格与加号等常见编码
        assert_eq!(percent_decode("a%20b%2Fc"), "a b/c");
    }

    #[test]
    fn keeps_malformed_and_plain_paths() {
        assert_eq!(percent_decode("plain/path.png"), "plain/path.png");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz"); // 非法十六进制原样保留
    }
}
