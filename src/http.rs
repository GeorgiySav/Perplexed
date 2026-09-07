use std::time::Duration;

pub const WEB_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
pub const LLM_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

pub const LLM_STALL_TIMEOUT: Duration = Duration::from_secs(120);

pub const USER_AGENT: &str = "Mozilla/5.0 (compatible; Perplexed/0.1)";

pub fn web_client() -> reqwest::Result<reqwest::Client> {
    web_client_with_timeout(WEB_REQUEST_TIMEOUT)
}

fn web_client_with_timeout(timeout: Duration) -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(timeout)
        .user_agent(USER_AGENT)
        .build()
}

pub fn llm_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(LLM_CONNECT_TIMEOUT)
        .read_timeout(LLM_STALL_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};

    fn drain_request(stream: &mut TcpStream) {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                return;
            }
            if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = v.trim().parse().unwrap_or(0);
            }
            if line == "\r\n" || line == "\n" {
                break;
            }
        }
        let mut body = vec![0u8; content_length];
        use std::io::Read;
        let _ = reader.read_exact(&mut body);
    }

    fn slow_streaming_server(chunk_count: usize, gap: Duration) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/api/chat", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            drain_request(&mut stream);
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\n\
                      Content-Type: application/x-ndjson\r\n\
                      Transfer-Encoding: chunked\r\n\r\n",
                )
                .unwrap();
            stream.flush().unwrap();
            for i in 0..chunk_count {
                std::thread::sleep(gap);
                let done = i == chunk_count - 1;
                let line = format!(
                    "{{\"message\":{{\"content\":\"tok\"}},\"done\":{}}}\n",
                    done
                );
                if stream
                    .write_all(format!("{:x}\r\n{}\r\n", line.len(), line).as_bytes())
                    .is_err()
                {
                    return;
                }
                if stream.flush().is_err() {
                    return;
                }
            }
            let _ = stream.write_all(b"0\r\n\r\n");
            let _ = stream.flush();
        });
        url
    }

    #[tokio::test]
    async fn llm_client_reads_a_stream_that_outlasts_the_web_timeout() {
        let chunks = 12;
        let gap = Duration::from_secs(1);
        assert!(
            gap * chunks as u32 > WEB_REQUEST_TIMEOUT,
            "test must outlast the web deadline to be meaningful"
        );
        let url = slow_streaming_server(chunks, gap);

        let body = llm_client()
            .unwrap()
            .post(&url)
            .json(&serde_json::json!({"model": "m", "messages": []}))
            .send()
            .await
            .expect("llm client must not abort a slow generation")
            .text()
            .await
            .expect("llm client must read the whole stream");

        assert_eq!(body.matches("\"done\"").count(), chunks);
    }

    #[tokio::test]
    async fn web_client_enforces_its_total_deadline() {
        let url = slow_streaming_server(10, Duration::from_secs(1));

        let err = web_client_with_timeout(Duration::from_millis(300))
            .unwrap()
            .get(&url)
            .send()
            .await
            .expect("headers arrive immediately")
            .text()
            .await
            .expect_err("a slow page must hit the web deadline");

        assert!(err.is_timeout(), "expected a timeout, got: {err}");
    }
}
