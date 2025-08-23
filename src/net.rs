use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use log::info;

use crate::app::LOCALHOST_IPV4;
const PORT_FILE: &str = "PORT";

use crate::utils::get_config_dir_path;

pub struct NetHandler {
    running: Arc<Mutex<bool>>,
    handler: thread::JoinHandle<()>,
}

impl NetHandler {
    pub fn new() -> Self {
        let running = Arc::new(Mutex::new(true));
        let running_clone = running.clone();
        let thread_handle = thread::spawn(move || {
            let listener = std::net::TcpListener::bind(format!("{}:0", LOCALHOST_IPV4)).unwrap();
            std::fs::write(
                format!(
                    "{}/{}/{}",
                    std::env::var("LOCALAPPDATA").unwrap(),
                    get_config_dir_path().to_str().unwrap(),
                    PORT_FILE
                ),
                listener.local_addr().unwrap().port().to_string(),
            )
            .unwrap();
            listener.set_nonblocking(true).unwrap();
            let running_lock = running_clone.lock();
            let running_val = *running_lock.unwrap();

            for stream in listener.incoming() {
                if !running_val {
                    return;
                }

                async_std::task::sleep(Duration::from_secs(1)).await;
                match stream {
                    Ok(mut stream) => {
                        info!(
                            "reading from stream: {}",
                            stream.peer_addr().unwrap().to_string()
                        );

                        // do something with the TcpStream
                        stream
                            .set_read_timeout(Some(Duration::from_millis(100)))
                            .unwrap();
                        let mut file_path = String::new();
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // wait until network socket is ready, typically implemented
                        // via platform-specific APIs such as epoll or IOCP
                        thread::sleep(Duration::from_millis(300));
                        continue;
                    }
                    Err(e) => panic!("encountered IO error: {e}"),
                }
            }
        });

        NetHandler {
            running,
            handler: thread_handle,
        }
    }

    pub fn shutdown(self) {
        *self.running.lock().unwrap() = false;
        self.handler.join().unwrap();
    }
}
