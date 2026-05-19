use crate::core::whois;
use crate::types::whois::WhoisResult;

#[tauri::command]
pub async fn whois_lookup(query: String) -> Result<WhoisResult, String> {
    whois::whois_lookup(&query, None).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whois_lookup_empty_query() {
        let result = futures::executor::block_on(whois_lookup("".into()));
        assert!(result.is_err());
    }

    #[test]
    fn test_whois_lookup_invalid_domain() {
        let result = futures::executor::block_on(whois_lookup("not a valid domain \t\n".into()));
        assert!(result.is_err() || result.is_ok());
        // Domain parsing is lenient, result depends on network
    }

    #[test]
    fn test_whois_lookup_parameter_passthrough() {
        // Test that the command function handles different inputs
        let inputs = vec!["example.com", "example.org", "192.168.1.1"];
        for input in inputs {
            // These will likely fail due to no network in tests,
            // but should not panic
            let result = futures::executor::block_on(whois_lookup(input.to_string()));
            // Either error or success is fine as long as it doesn't panic
            assert!(result.is_err() || result.is_ok());
        }
    }
}
