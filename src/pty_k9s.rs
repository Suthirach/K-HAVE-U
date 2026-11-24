use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex, mpsc};

#[derive(Clone, Debug)]
pub struct K9sScreen {
    pub lines: Vec<String>,
    pub running: bool,
    pub status: String,
}

impl Default for K9sScreen {
    fn default() -> Self {
        Self {
            lines: vec![crate::tr!("กด \"Start / Connect K9s\" เพื่อเปิด K9s CLI ในหน้านี้", "Press \"Start / Connect K9s\" to open the K9s CLI here").into()],
            running: false,
            status: crate::tr!("ยังไม่เชื่อมต่อ", "Not connected").into(),
        }
    }
}

pub struct K9sPty {
    writer: Option<Box<dyn Write + Send>>,
    _master: Option<Box<dyn MasterPty + Send>>,
    child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
}

impl K9sPty {
    pub fn new() -> Self {
        Self {
            writer: None,
            _master: None,
            child: None,
        }
    }

    pub fn is_running(&self) -> bool {
        self.writer.is_some()
    }

    pub fn stop(&mut self) {
        self.writer = None;
        self._master = None;
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
        }
    }

    pub fn start(
        &mut self,
        namespace: &str,
        tx: mpsc::Sender<K9sScreen>,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) {
        self.stop();

        let pty_system = native_pty_system();
        let pair = match pty_system.openpty(PtySize {
            rows: 32,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            Ok(p) => p,
            Err(e) => {
                let _ = tx.send(K9sScreen {
                    lines: vec![crate::tf!("ไม่สามารถเปิด PTY ได้: {e}", "Could not open a PTY: {e}")],
                    running: false,
                    status: "PTY Error".into(),
                });
                wake();
                return;
            }
        };

        let mut cmd = CommandBuilder::new("k9s");
        cmd.arg("-n");
        cmd.arg(namespace);

        let child = match pair.slave.spawn_command(cmd) {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(K9sScreen {
                    lines: vec![
                        crate::tf!("ไม่สามารถรัน k9s ได้: {e}", "Could not run k9s: {e}"),
                        crate::tr!("ตรวจสอบว่าติดตั้ง k9s และอยู่ใน PATH แล้ว", "Check that k9s is installed and on PATH").into(),
                    ],
                    running: false,
                    status: "Spawn Error".into(),
                });
                wake();
                return;
            }
        };

        let mut reader = match pair.master.try_clone_reader() {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.send(K9sScreen {
                    lines: vec![crate::tf!("อ่าน output จาก PTY ไม่ได้: {e}", "Could not read PTY output: {e}")],
                    running: false,
                    status: "Reader Error".into(),
                });
                wake();
                return;
            }
        };

        let writer = match pair.master.take_writer() {
            Ok(w) => w,
            Err(e) => {
                let _ = tx.send(K9sScreen {
                    lines: vec![crate::tf!("เขียน input เข้า PTY ไม่ได้: {e}", "Could not write to the PTY: {e}")],
                    running: false,
                    status: "Writer Error".into(),
                });
                wake();
                return;
            }
        };

        self.writer = Some(writer);
        self._master = Some(pair.master);
        self.child = Some(child);

        let ns_label = namespace.to_string();
        let parser = Arc::new(Mutex::new(vt100::Parser::new(32, 120, 0)));

        let _ = tx.send(K9sScreen {
            lines: vec![crate::tf!("กำลังเปิด k9s -n {ns_label} ...", "Opening k9s -n {ns_label} ...")],
            running: true,
            status: format!("Connected: {ns_label}"),
        });
        wake();

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => {
                        let _ = tx.send(K9sScreen {
                            lines: vec![crate::tf!("k9s (-n {ns_label}) ปิดการทำงานแล้ว", "k9s (-n {ns_label}) exited")],
                            running: false,
                            status: "Disconnected".into(),
                        });
                        wake();
                        break;
                    }
                    Ok(n) => {
                        if let Ok(mut p) = parser.lock() {
                            p.process(&buf[..n]);
                            let screen = p.screen();
                            let contents = screen.contents();
                            let lines: Vec<String> = contents.lines().map(|l| l.to_string()).collect();
                            let _ = tx.send(K9sScreen {
                                lines,
                                running: true,
                                status: format!("Live K9s: {ns_label}"),
                            });
                            wake();
                        }
                    }
                }
            }
        });
    }

    pub fn send_bytes(&mut self, bytes: &[u8]) {
        if let Some(w) = &mut self.writer {
            let _ = w.write_all(bytes);
            let _ = w.flush();
        }
    }
}
