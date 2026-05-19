use crate::core::http_check;
use crate::types::http_check::HttpCheckResult;

#[tauri::command]
pub async fn http_check(url: String) -> Result<HttpCheckResult, String> {
    http_check::check_http(&url, 10).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_check_empty_url() {
        let result = futures::executor::block_on(http_check("".into()));
        assert!(result.is_err());
    }

    #[test]
    fn test_http_check_invalid_protocol() {
        let result = futures::executor::block_on(http_check("ftp://example.com".into()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("URL must start with"));
    }

    #[test]
    fn test_http_check_no_protocol() {
        let result = futures::executor::block_on(http_check("example.com".into()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("URL must start with"));
    }

    #[test]
    fn test_http_check_https_not_supported() {
        let result = futures::executor::block_on(http_check("https://example.com".into()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("TLS"));
    }

    #[test]
    fn test_http_check_timeout_short_url() {
        let result = futures::executor::block_on(http_check("http://10.255.255.1:81".into()));
        // Expected to fail (timeout or connection refused) but not panic
        assert!(result.is_err());
    }
}
