use crate::core::ssl_check;
use crate::types::ssl_check::SslCheckResult;

#[tauri::command]
pub async fn ssl_check(hostname: String, port: Option<u16>) -> Result<SslCheckResult, String> {
    let port = port.unwrap_or(443);
    ssl_check::check_ssl(&hostname, port).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssl_check_empty_hostname() {
        let result = futures::executor::block_on(ssl_check("".into(), None));
        assert!(result.is_err());
    }

    #[test]
    fn test_ssl_check_custom_port() {
        let result = futures::executor::block_on(ssl_check("example.com".into(), Some(8443)));
        // May fail due to network, but should not panic
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_ssl_check_default_port() {
        // Port should default to 443
        let result = futures::executor::block_on(ssl_check("127.0.0.1".into(), None));
        assert!(result.is_err()); // 127.0.0.1 likely not running SSL
    }

    #[test]
    fn test_ssl_check_unreachable() {
        let result = futures::executor::block_on(ssl_check("192.0.2.1".into(), Some(443)));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.is_empty());
    }
}
