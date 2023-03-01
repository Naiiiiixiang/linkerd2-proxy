use crate::*;
use linkerd2_proxy_api::{self as api};
use policy::outbound::{self, proxy_protocol};

#[tokio::test]
async fn default_http1_route() {
    let _trace = trace_init();

    const AUTHORITY: &str = "policy.test.svc.cluster.local";

    let srv = server::http1().route("/", "hello h1").run().await;
    let ctrl = controller::new();
    let dst = format!("{AUTHORITY}:{}", srv.addr.port());
    let dest_tx = ctrl.destination_tx(&dst);
    dest_tx.send_addr(srv.addr);
    let _profile_tx = ctrl.profile_tx_default(srv.addr, AUTHORITY);
    let policy = controller::policy()
        // stop the admin server from entering an infinite retry loop
        .with_inbound_default(policy::all_unauthenticated())
        .outbound_default(srv.addr, &dst);

    let proxy = proxy::new()
        .controller(ctrl.run().await)
        .policy(policy.run().await)
        .outbound(srv)
        .run()
        .await;
    let client = client::http1(proxy.outbound, AUTHORITY);

    assert_eq!(client.get("/").await, "hello h1");
    // ensure panics from the server are propagated
    proxy.join_servers().await;
}

#[tokio::test]
async fn empty_http1_route() {
    let _trace = trace_init();

    const AUTHORITY: &str = "policy.test.svc.cluster.local";

    let srv = server::http1().route("/", "hello h1").run().await;
    let ctrl = controller::new();

    let dst = format!("{AUTHORITY}:{}", srv.addr.port());
    let dst_tx = ctrl.destination_tx(&dst);
    dst_tx.send_addr(srv.addr);
    let _profile_tx = ctrl.profile_tx_default(srv.addr, AUTHORITY);
    let policy = controller::policy()
        // stop the admin server from entering an infinite retry loop
        .with_inbound_default(policy::all_unauthenticated())
        .outbound(
            srv.addr,
            outbound::OutboundPolicy {
                protocol: Some(outbound::ProxyProtocol {
                    kind: Some(proxy_protocol::Kind::Detect(proxy_protocol::Detect {
                        timeout: Some(Duration::from_secs(10).try_into().unwrap()),
                        http1: Some(proxy_protocol::Http1 {
                            http_routes: vec![outbound::HttpRoute {
                                metadata: Some(api::meta::Metadata {
                                    kind: Some(api::meta::metadata::Kind::Resource(
                                        api::meta::Resource {
                                            group: "gateway.networking.k8s.io".to_string(),
                                            kind: "HTTPRoute".to_string(),
                                            name: "empty".to_string(),
                                            namespace: "test".to_string(),
                                            section: "".to_string(),
                                        },
                                    )),
                                }),
                                hosts: Vec::new(),
                                rules: Vec::new(),
                            }],
                        }),
                        http2: Some(proxy_protocol::Http2 {
                            http_routes: vec![policy::outbound_default_route(&dst)],
                        }),
                    })),
                }),
                ..policy::outbound_default(dst)
            },
        );

    let proxy = proxy::new()
        .controller(ctrl.run().await)
        .policy(policy.run().await)
        .outbound(srv)
        .run()
        .await;
    let client = client::http1(proxy.outbound, AUTHORITY);
    let rsp = client.request(client.request_builder("/")).await.unwrap();
    assert_eq!(rsp.status(), http::StatusCode::NOT_FOUND);

    // ensure panics from the server are propagated
    proxy.join_servers().await;
}

#[tokio::test]
async fn default_http2_route() {
    let _trace = trace_init();

    const AUTHORITY: &str = "policy.test.svc.cluster.local";

    let srv = server::http2().route("/", "hello h2").run().await;
    let ctrl = controller::new();
    let dst = format!("{AUTHORITY}:{}", srv.addr.port());
    let dest_tx = ctrl.destination_tx(&dst);
    dest_tx.send_addr(srv.addr);
    let _profile_tx = ctrl.profile_tx_default(srv.addr, AUTHORITY);
    let policy = controller::policy()
        // stop the admin server from entering an infinite retry loop
        .with_inbound_default(policy::all_unauthenticated())
        .outbound_default(srv.addr, &dst);

    let proxy = proxy::new()
        .controller(ctrl.run().await)
        .policy(policy.run().await)
        .outbound(srv)
        .run()
        .await;
    let client = client::http2(proxy.outbound, AUTHORITY);

    assert_eq!(client.get("/").await, "hello h2");
    // ensure panics from the server are propagated
    proxy.join_servers().await;
}

#[tokio::test]
async fn empty_http2_route() {
    let _trace = trace_init();

    const AUTHORITY: &str = "policy.test.svc.cluster.local";

    let srv = server::http2().route("/", "hello h2").run().await;
    let ctrl = controller::new();

    let dst = format!("{AUTHORITY}:{}", srv.addr.port());
    let dst_tx = ctrl.destination_tx(&dst);
    dst_tx.send_addr(srv.addr);
    let _profile_tx = ctrl.profile_tx_default(srv.addr, AUTHORITY);
    let policy = controller::policy()
        // stop the admin server from entering an infinite retry loop
        .with_inbound_default(policy::all_unauthenticated())
        .outbound(
            srv.addr,
            outbound::OutboundPolicy {
                protocol: Some(outbound::ProxyProtocol {
                    kind: Some(proxy_protocol::Kind::Detect(proxy_protocol::Detect {
                        timeout: Some(Duration::from_secs(10).try_into().unwrap()),
                        http1: Some(proxy_protocol::Http1 {
                            http_routes: vec![policy::outbound_default_route(&dst)],
                        }),
                        http2: Some(proxy_protocol::Http2 {
                            http_routes: vec![outbound::HttpRoute {
                                metadata: Some(api::meta::Metadata {
                                    kind: Some(api::meta::metadata::Kind::Resource(
                                        api::meta::Resource {
                                            group: "gateway.networking.k8s.io".to_string(),
                                            kind: "HTTPRoute".to_string(),
                                            name: "empty".to_string(),
                                            namespace: "test".to_string(),
                                            section: "".to_string(),
                                        },
                                    )),
                                }),
                                hosts: Vec::new(),
                                rules: Vec::new(),
                            }],
                        }),
                    })),
                }),
                ..policy::outbound_default(dst)
            },
        );

    let proxy = proxy::new()
        .controller(ctrl.run().await)
        .policy(policy.run().await)
        .outbound(srv)
        .run()
        .await;
    let client = client::http2(proxy.outbound, AUTHORITY);
    let rsp = client.request(client.request_builder("/")).await.unwrap();
    assert_eq!(rsp.status(), http::StatusCode::NOT_FOUND);

    // ensure panics from the server are propagated
    proxy.join_servers().await;
}
