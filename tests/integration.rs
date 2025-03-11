#[macro_use]
extern crate rocket;

use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use rocket::{http::ContentType, local::asynchronous::Client};
use rocket_prometheus::PrometheusMetrics;
use serde_json::json;

type NameCounter = Family<NameLabel, Counter>;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct NameLabel {
    name: String,
}

mod routes {
    use rocket::serde::json::Json;
    use rocket::State;
    use serde::Deserialize;

    use super::{NameCounter, NameLabel};

    #[get("/hello/<name>?<caps>")]
    pub fn hello(name: &str, caps: Option<bool>, name_counter: &State<NameCounter>) -> String {
        let name = caps
            .unwrap_or_default()
            .then(|| name.to_uppercase())
            .unwrap_or_else(|| name.to_string());

        name_counter
            .get_or_create(&NameLabel { name: name.clone() })
            .inc();

        format!("Hello, {}!", name)
    }

    #[derive(Deserialize)]
    pub struct Person {
        age: u8,
    }

    #[post("/hello/<name>?<caps>", format = "json", data = "<person>")]
    pub fn hello_post(name: String, person: Json<Person>, caps: Option<bool>) -> String {
        let name = caps
            .unwrap_or_default()
            .then(|| name.to_uppercase())
            .unwrap_or_else(|| name.to_string());

        format!("Hello, {} year old named {}!", person.age, name)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions::assert_eq;

    #[rocket::async_test]
    async fn test_basic() {
        let prometheus = PrometheusMetrics::new()
            .with_request_filter(|request| request.uri().path() != "/metrics");

        let name_counter = NameCounter::default();

        {
            let mut registry = prometheus.registry().lock().await;
            registry.register("name_counter", "Count of names", name_counter.clone());
        }

        let rocket = rocket::build()
            .attach(prometheus.clone())
            .manage(name_counter)
            .mount("/", routes![routes::hello, routes::hello_post])
            .mount("/metrics", prometheus);
        let client = Client::untracked(rocket)
            .await
            .expect("valid rocket instance");
        client.get("/hello/foo").dispatch().await;
        client.get("/hello/foo").dispatch().await;
        client.get("/hello/bar").dispatch().await;
        client.get("/metrics").dispatch().await;
        client
            .post("/hello/bar")
            .header(ContentType::JSON)
            .body(serde_json::to_string(&json!({"age": 50})).unwrap())
            .dispatch()
            .await;
        let metrics = client.get("/metrics").dispatch().await;
        let response = metrics.into_string().await.unwrap();

        let lines = response.lines().collect::<Vec<_>>();
        let mut first = lines[..4].to_vec(); // skip EOF
        first.sort();
        let mut second = lines[5..9].to_vec();
        second.sort();

        let mut third = lines[9..lines.len() - 1].to_vec(); // skip eof
        third.sort();

        assert_eq!(
            first,
            vec![
                r#"# HELP name_counter Count of names."#,
                r#"# TYPE name_counter counter"#,
                r#"name_counter_total{name="bar"} 1"#,
                r#"name_counter_total{name="foo"} 2"#,
            ]
        );
        assert_eq!(
            second,
            vec![
                r#"# HELP rocket_http_requests_total Total number of HTTP requests."#,
                r#"# TYPE rocket_http_requests_total counter"#,
                r#"rocket_http_requests_total_total{endpoint="/hello/<name>?<caps>",status="200",method="GET"} 3"#,
                r#"rocket_http_requests_total_total{endpoint="/hello/<name>?<caps>",status="200",method="POST"} 1"#,
            ]
        );

        assert_eq!(
            &third[..third.len() - 2], // skip two variable request
            &vec![
                "# HELP rocket_http_requests_duration_seconds HTTP request duration in seconds for all requests.",
                "# TYPE rocket_http_requests_duration_seconds histogram",
                "rocket_http_requests_duration_seconds_bucket{le=\"+Inf\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"GET\"} 3",
                "rocket_http_requests_duration_seconds_bucket{le=\"+Inf\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"POST\"} 1",
                "rocket_http_requests_duration_seconds_bucket{le=\"0.005\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"GET\"} 3",
                "rocket_http_requests_duration_seconds_bucket{le=\"0.005\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"POST\"} 1",
                "rocket_http_requests_duration_seconds_bucket{le=\"0.01\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"GET\"} 3",
                "rocket_http_requests_duration_seconds_bucket{le=\"0.01\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"POST\"} 1",
                "rocket_http_requests_duration_seconds_bucket{le=\"0.025\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"GET\"} 3",
                "rocket_http_requests_duration_seconds_bucket{le=\"0.025\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"POST\"} 1",
                "rocket_http_requests_duration_seconds_bucket{le=\"0.05\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"GET\"} 3",
                "rocket_http_requests_duration_seconds_bucket{le=\"0.05\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"POST\"} 1",
                "rocket_http_requests_duration_seconds_bucket{le=\"0.1\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"GET\"} 3",
                "rocket_http_requests_duration_seconds_bucket{le=\"0.1\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"POST\"} 1",
                "rocket_http_requests_duration_seconds_bucket{le=\"0.25\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"GET\"} 3",
                "rocket_http_requests_duration_seconds_bucket{le=\"0.25\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"POST\"} 1",
                "rocket_http_requests_duration_seconds_bucket{le=\"0.5\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"GET\"} 3",
                "rocket_http_requests_duration_seconds_bucket{le=\"0.5\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"POST\"} 1",
                "rocket_http_requests_duration_seconds_bucket{le=\"1.0\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"GET\"} 3",
                "rocket_http_requests_duration_seconds_bucket{le=\"1.0\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"POST\"} 1",
                "rocket_http_requests_duration_seconds_bucket{le=\"10.0\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"GET\"} 3",
                "rocket_http_requests_duration_seconds_bucket{le=\"10.0\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"POST\"} 1",
                "rocket_http_requests_duration_seconds_bucket{le=\"2.5\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"GET\"} 3",
                "rocket_http_requests_duration_seconds_bucket{le=\"2.5\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"POST\"} 1",
                "rocket_http_requests_duration_seconds_bucket{le=\"5.0\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"GET\"} 3",
                "rocket_http_requests_duration_seconds_bucket{le=\"5.0\",endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"POST\"} 1",
                "rocket_http_requests_duration_seconds_count{endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"GET\"} 3",
                "rocket_http_requests_duration_seconds_count{endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"POST\"} 1",
                // these vary
                // "rocket_http_requests_duration_seconds_sum{endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"GET\"} 0.000191542",
                // "rocket_http_requests_duration_seconds_sum{endpoint=\"/hello/<name>?<caps>\",status=\"200\",method=\"POST\"} 0.000075958"
            ],
        );
    }
}
