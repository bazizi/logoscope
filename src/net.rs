use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::io::Read;

use log::info;

use crate::app::App;
use crate::{CONFIGS_PATH, PORT_FILE, LOCALHOST_IPV4};

pub struct NetHandler{
    running: Arc<Mutex<bool>>,
    handler: thread::JoinHandle<()>
}

impl NetHandler
{
    pub fn new(app: Arc<Mutex<App>>) -> Self
    {
        let app_clone = Arc::clone(&app);
        let running = Arc::new(Mutex::new(true));
        let running_clone = running.clone();
        let thread_handle = thread::spawn(move || {
            let listener = std::net::TcpListener::bind(format!("{}:0", LOCALHOST_IPV4)).unwrap();
            std::fs::write(
                format!(
                    "{}/{}/{}",
                    std::env::var("LOCALAPPDATA").unwrap(),
                    CONFIGS_PATH,
                    PORT_FILE
                ),
                listener.local_addr().unwrap().port().to_string(),
            )
            .unwrap();
            listener.set_nonblocking(true).unwrap();
            for stream in listener.incoming() {
                if !app_clone.lock().unwrap().running() || !*running_clone.lock().unwrap() {
                    break;
                }
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
                        stream.read_to_string(&mut file_path).unwrap();
                        let table_items = crate::tab::TableItems {
                            data: crate::parser::parse_log_by_path(&file_path).unwrap_or_default(),
                            selected_item_index: 0,
                        };
                        let mut app_lock = app_clone.lock().unwrap();
                        app_lock.tabs_mut().push(crate::tab::Tab::new(
                            file_path.to_string(),
                            table_items,
                            crate::tab::TabType::Normal,
                        ));
                        *app_lock.selected_tab_index_mut() = app_lock.tabs().len() - 1;
                        app_lock.reload_combined_tab();
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

        NetHandler{running, handler: thread_handle}
    }

    pub fn shutdown(self) {
        *self.running.lock().unwrap() = false;
        self.handler.join().unwrap();
    }
}
