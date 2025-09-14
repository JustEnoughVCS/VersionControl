use crate::example_handle::{ExampleClientHandle, ExampleServerHandle};
use tcp_connection::target::TcpServerTarget;

#[test]
fn test_tcp_test_target_build() {
    let host = "127.0.0.1:8080";

    // Test build target by string
    let Ok(target) = TcpServerTarget::<ExampleClientHandle, ExampleServerHandle>::from_str(host)
    else {
        panic!("Test target built from a target addr `{}`", host);
    };
    assert_eq!(target.to_string(), "127.0.0.1:8080");
}

#[tokio::test]
async fn test_tcp_test_target_build_domain() {
    let host = "localhost";

    // Test build target by DomainName and Connection
    let Ok(target) =
        TcpServerTarget::<ExampleClientHandle, ExampleServerHandle>::from_domain(host).await
    else {
        panic!("Test target built from a domain named `{}`", host);
    };

    // Test into string
    assert_eq!(target.to_string(), "127.0.0.1:8080");
}            default_port,
