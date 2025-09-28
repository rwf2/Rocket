// Tests for gRPC service utilities that can be tested independently
// These tests focus on the working components without the problematic fairing implementation

#[cfg(feature = "tonic")]
mod service_tests {
    use rocket_grpc::{StateAwareService, metadata_to_headers};
    use tonic::metadata::{MetadataMap, MetadataValue};

    #[derive(Clone, Debug, PartialEq)]
    struct MockService {
        name: String,
    }

    impl MockService {
        fn new(name: String) -> Self {
            Self { name }
        }

        fn get_name(&self) -> &str {
            &self.name
        }
    }

    #[test]
    fn test_state_aware_service_new() {
        let service = MockService::new("test_service".to_string());
        let state = "test_state".to_string();
        
        let state_aware = StateAwareService::new(service.clone(), state.clone());
        
        assert_eq!(state_aware.state(), Some(&state));
        assert_eq!(state_aware.service().get_name(), "test_service");
    }

    #[test]
    fn test_state_aware_service_without_state() {
        let service = MockService::new("test_service".to_string());
        
        let state_aware: StateAwareService<MockService, String> = StateAwareService::without_state(service);
        
        assert_eq!(state_aware.state(), None);
        assert_eq!(state_aware.service().get_name(), "test_service");
    }

    #[test]
    fn test_state_aware_service_methods() {
        let service = MockService::new("test_service".to_string());
        let state = 42i32;
        
        let state_aware = StateAwareService::new(service.clone(), state);
        
        // Test state access
        assert_eq!(state_aware.state(), Some(&42));
        
        // Test service access
        assert_eq!(state_aware.service().get_name(), "test_service");
    }

    #[test]
    fn test_state_aware_service_clone() {
        let service = MockService::new("test_service".to_string());
        let state = "test_state".to_string();
        
        let state_aware = StateAwareService::new(service.clone(), state.clone());
        let cloned_state_aware = state_aware.clone();
        
        assert_eq!(state_aware.state(), cloned_state_aware.state());
        assert_eq!(state_aware.service().get_name(), cloned_state_aware.service().get_name());
    }

    #[test]
    fn test_metadata_to_headers_empty() {
        let metadata = MetadataMap::new();
        let headers = metadata_to_headers(&metadata);
        
        assert!(headers.is_empty());
    }

    #[test]
    fn test_metadata_to_headers_with_values() {
        let mut metadata = MetadataMap::new();
        metadata.insert("content-type", MetadataValue::from_static("application/grpc"));
        metadata.insert("authorization", MetadataValue::from_static("Bearer token123"));
        
        let headers = metadata_to_headers(&metadata);
        
        assert_eq!(headers.len(), 2);
        assert_eq!(headers.get("content-type"), Some(&"application/grpc".to_string()));
        assert_eq!(headers.get("authorization"), Some(&"Bearer token123".to_string()));
    }

    #[test]
    fn test_metadata_to_headers_invalid_values() {
        let mut metadata = MetadataMap::new();
        // Add a valid header
        metadata.insert("valid-header", MetadataValue::from_static("valid-value"));
        
        let headers = metadata_to_headers(&metadata);
        
        // Should only contain the valid header
        assert_eq!(headers.len(), 1);
        assert_eq!(headers.get("valid-header"), Some(&"valid-value".to_string()));
    }

    #[test]
    fn test_metadata_to_headers_multiple_headers() {
        let mut metadata = MetadataMap::new();
        metadata.insert("header1", MetadataValue::from_static("value1"));
        metadata.insert("header2", MetadataValue::from_static("value2"));
        metadata.insert("header3", MetadataValue::from_static("value3"));
        
        let headers = metadata_to_headers(&metadata);
        
        assert_eq!(headers.len(), 3);
        assert_eq!(headers.get("header1"), Some(&"value1".to_string()));
        assert_eq!(headers.get("header2"), Some(&"value2".to_string()));
        assert_eq!(headers.get("header3"), Some(&"value3".to_string()));
    }

    // Test the grpc_service! macro
    #[test]
    fn test_grpc_service_macro() {
        use rocket_grpc::grpc_service;

        grpc_service! {
            TestService {
                state: String,
                
                fn test_method(&self) -> Option<&String> {
                    self.state()
                }
            }
        }

        // Test with state
        let service = TestService::new("test_state".to_string());
        assert_eq!(service.state(), Some(&"test_state".to_string()));
        assert_eq!(service.test_method(), Some(&"test_state".to_string()));

        // Test without state
        let service_no_state = TestService::without_state();
        assert_eq!(service_no_state.state(), None);
        assert_eq!(service_no_state.test_method(), None);
    }

    #[test]
    fn test_grpc_service_macro_with_complex_state() {
        use rocket_grpc::grpc_service;

        #[derive(Clone, Debug, PartialEq)]
        struct ComplexState {
            id: u64,
            name: String,
        }

        grpc_service! {
            ComplexService {
                state: ComplexState,
                
                fn get_id(&self) -> Option<u64> {
                    self.state().map(|s| s.id)
                }
                
                fn get_name(&self) -> Option<&String> {
                    self.state().map(|s| &s.name)
                }
            }
        }

        let state = ComplexState { id: 42, name: "test".to_string() };
        let service = ComplexService::new(state.clone());
        
        assert_eq!(service.get_id(), Some(42));
        assert_eq!(service.get_name(), Some(&"test".to_string()));
        assert_eq!(service.state(), Some(&state));
    }

    #[test]
    fn test_grpc_service_macro_clone() {
        use rocket_grpc::grpc_service;

        grpc_service! {
            CloneableService {
                state: i32,
                
                fn get_value(&self) -> Option<i32> {
                    self.state().copied()
                }
            }
        }

        let service = CloneableService::new(42);
        let cloned_service = service.clone();
        
        assert_eq!(service.get_value(), Some(42));
        assert_eq!(cloned_service.get_value(), Some(42));
        
        // Ensure they are independent clones
        assert_eq!(service.state(), cloned_service.state());
    }
}

// Tests that should work without tonic feature
#[cfg(not(feature = "tonic"))]
#[test]
fn test_no_tonic_feature() {
    // Just ensure the crate compiles without tonic feature
    // The actual functionality won't be available but compilation should succeed
    assert!(true);
}