// bankasset:// 自定义协议：bankasset://<bank_id>/<相对路径> → AppData/banks/<bank_id>/assets/
// 统一解析内置/导入题库图片（设计方案 V1.1 §2.2）
use tauri::http::{Request, Response, StatusCode};
use tauri::Manager;

pub fn handle<T: tauri::Runtime>(app: &tauri::AppHandle<T>, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let path = request.uri().path().to_string(); // 形如 /<bank_id>/<rel...>
    let parts: Vec<&str> = path.trim_start_matches('/').splitn(2, '/').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return resp(StatusCode::BAD_REQUEST, "text/plain", b"bad request".to_vec());
    }
    let (bank_id, rel) = (parts[0], parts[1]);
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
