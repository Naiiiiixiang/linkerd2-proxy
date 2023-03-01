use crate::*;
use linkerd2_proxy_api::{self as api};
use policy::outbound::{self, distribution, proxy_protocol};

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

#[tokio::test]
async fn header_based_routing() {
    let _trace = trace_init();

    const AUTHORITY_WORLD: &str = "world.test.svc.cluster.local";
    const AUTHORITY_SF: &str = "sf.test.svc.cluster.local";
    const AUTHORITY_AUSTIN: &str = "austin.test.svc.cluster.local";
    const HEADER: &str = "x-hello-city";

    let srv = server::http1().route("/", "hello world!").run().await;
    let srv_sf = server::http1()
        .route("/", "hello san francisco!")
        .run()
        .await;
    let srv_austin = server::http1().route("/", "hello austin!").run().await;
    let ctrl = controller::new();

    let dst_world = format!("{AUTHORITY_WORLD}:{}", srv.addr.port());
    let dst_sf = format!("{AUTHORITY_SF}:{}", srv_sf.addr.port());
    let dst_austin = format!("{AUTHORITY_AUSTIN}:{}", srv_sf.addr.port());

    let dst_world_tx = ctrl.destination_tx(&dst_world);
    dst_world_tx.send_addr(srv.addr);
    let dst_sf_tx = ctrl.destination_tx(&dst_sf);
    dst_sf_tx.send_addr(srv_sf.addr);
    let dst_austin_tx = ctrl.destination_tx(&dst_austin);
    dst_austin_tx.send_addr(srv_austin.addr);

    let _profile_tx = ctrl.profile_tx_default(srv.addr, AUTHORITY_WORLD);

    let route = outbound::HttpRoute {
        metadata: Some(api::meta::Metadata {
            kind: Some(api::meta::metadata::Kind::Resource(api::meta::Resource {
                group: "gateway.networking.k8s.io".to_string(),
                kind: "HTTPRoute".to_string(),
                name: "hello".to_string(),
                namespace: "test".to_string(),
                section: "".to_string(),
            })),
        }),
        hosts: Vec::new(),
        rules: vec![
            // generic hello world
            outbound::http_route::Rule {
                matches: Vec::new(),
                filters: Vec::new(),
                backends: Some(outbound::Distribution {
                    distribution: Some(distribution::Distribution::FirstAvailable(
                        distribution::FirstAvailable {
                            backends: vec![policy::backend(&dst_world, 1)],
                        },
                    )),
                }),
            },
            // x-hello-city: sf | x-hello-city: san francisco
            outbound::http_route::Rule {
                matches: vec![api::http_route::HttpRouteMatch {
                    headers: vec![api::http_route::HeaderMatch {
                        name: HEADER.to_string(),
                        value: Some(api::http_route::header_match::Value::Regex(
                            "sf|san francisco".to_string(),
                        )),
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                filters: Vec::new(),
                backends: Some(outbound::Distribution {
                    distribution: Some(distribution::Distribution::FirstAvailable(
                        distribution::FirstAvailable {
                            backends: vec![policy::backend(&dst_sf, 1)],
                        },
                    )),
                }),
            },
            // x-hello-city: austin
            outbound::http_route::Rule {
                matches: vec![api::http_route::HttpRouteMatch {
                    headers: vec![api::http_route::HeaderMatch {
                        name: HEADER.to_string(),
                        value: Some(api::http_route::header_match::Value::Exact(
                            "austin".to_string().into_bytes(),
                        )),
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                filters: Vec::new(),
                backends: Some(outbound::Distribution {
                    distribution: Some(distribution::Distribution::FirstAvailable(
                        distribution::FirstAvailable {
                            backends: vec![policy::backend(&dst_austin, 1)],
                        },
                    )),
                }),
            },
        ],
    };

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
                            http_routes: vec![route.clone()],
                        }),
                        http2: Some(proxy_protocol::Http2 {
                            http_routes: vec![route],
                        }),
                    })),
                }),
                ..policy::outbound_default(dst_world)
            },
        );

    let proxy = proxy::new()
        .controller(ctrl.run().await)
        .policy(policy.run().await)
        .outbound(srv)
        .run()
        .await;

    let client = client::http1(proxy.outbound, AUTHORITY_WORLD);

    let req = move |header: Option<&str>| {
        let mut builder = client.request_builder("/");
        if let Some(header) = header {
            tracing::info!("GET / {HEADER}: {header}");
            builder = builder.header(HEADER, header);
        } else {
            tracing::info!("GET / (no header)");
        }
        let fut = client.request(builder);
        async move {
            let res = fut.await.expect("request");
            tracing::info!(?res);
            assert!(
                res.status().is_success(),
                "client.get('/') expects 2xx, got \"{}\"",
                res.status(),
            );
            let stream = res.into_parts().1;
            http_util::body_to_string(stream).await.unwrap()
        }
    };

    assert_eq!(req(None).await, "hello world!");

    // matches SF route
    assert_eq!(req(Some("sf")).await, "hello san francisco!");

    // unknown header value matches default route
    assert_eq!(req(Some("paris")).await, "hello world!");

    // matches austin route
    assert_eq!(req(Some("austin")).await, "hello austin!");

    // also matches sf route regex
    assert_eq!(req(Some("san francisco")).await, "hello san francisco!");

    // ensure panics from the server are propagated
    proxy.join_servers().await;
}
