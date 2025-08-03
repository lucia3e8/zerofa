use dotenv::dotenv;
use imap::types::Fetch;
use native_tls::TlsConnector;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use tiny_http::{Response, Server};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TwoFactorCode {
    code: String,
    service: String,
    phrase: String,
}

fn save_code(code: &TwoFactorCode) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(code)?;
    fs::write("code.json", json)?;
    Ok(())
}

fn load_code() -> Option<TwoFactorCode> {
    match fs::read_to_string("code.json") {
        Ok(json) => serde_json::from_str(&json).ok(),
        Err(_) => None,
    }
}


fn main() {
    dotenv().ok();

    let phrases = &["magic beans", "smiling cops", "new friends", "caught fishes",
        "pet dogs", "meowing catgirls", "angry chefs", "ready scones"];

    let imap_server = env::var("IMAP_SERVER").expect("IMAP_SERVER not set");
    let imap_port = env::var("IMAP_PORT")
        .expect("IMAP_PORT not set")
        .parse::<u16>()
        .expect("IMAP_PORT must be a number");
    let username = env::var("IMAP_USERNAME").expect("IMAP_USERNAME not set");
    let password = env::var("IMAP_PASSWORD").expect("IMAP_PASSWORD not set");
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    // Shared storage for the latest 2FA code, load from disk if available
    let initial_code = load_code();
    if let Some(ref code) = initial_code {
        println!("Loaded existing 2FA code from disk: {} {}", code.code, code.phrase);
    }
    let latest_code: Arc<Mutex<Option<TwoFactorCode>>> = Arc::new(Mutex::new(initial_code));
    let latest_code_http = Arc::clone(&latest_code);

    // Start HTTP server in a separate thread
    let port_clone = port.clone();
    thread::spawn(move || {
        let addr = format!("0.0.0.0:{}", port_clone);
        let server = Server::http(&addr).expect("Failed to start HTTP server");
        println!("HTTP server listening on http://{}", addr);

        for request in server.incoming_requests() {
            let response = match latest_code_http.lock() {
                Ok(guard) => {
                    match &*guard {
                        // claude: pick a phrase from the `phrases` array at random here instead of
                        // the serice name
                        Some(code) => Response::from_string(format!("{} {}", code.code, code.phrase)),
                        None => Response::from_string("000000 sleeping frogs"),
                    }
                }
                Err(_) => Response::from_string("Error accessing 2FA code"),
            };

            let _ = request.respond(response);
        }
    });

    println!("Connecting to {}:{}...", imap_server, imap_port);

    let tls = TlsConnector::builder()
        .build()
        .expect("Failed to create TLS connector");

    let client = imap::connect((&imap_server[..], imap_port), &imap_server, &tls)
        .expect("Failed to connect to IMAP server");

    let mut imap_session = client
        .login(&username, &password)
        .expect("Failed to login");

    println!("Connected and authenticated successfully!");

    imap_session.select("INBOX").expect("Failed to select INBOX");

    // Get the highest UID of existing messages to know where to start
    let exists = imap_session.search("ALL").expect("Failed to search all messages");
    let last_seen_uid = exists.iter().max().copied().unwrap_or(0);
    println!("Starting from UID: {}", last_seen_uid + 1);

    loop {
        println!("Waiting for new emails (IDLE mode)...");

        let idle = imap_session.idle().expect("Failed to create IDLE handle");

        match idle.wait_keepalive() {
            Ok(()) => {
                println!("New email activity detected!");

                // Only fetch messages with UID greater than our starting point
                let search_query = format!("UID {}:*", last_seen_uid + 1);
                let messages = imap_session.search(&search_query).expect("Failed to search");

                for uid in messages.iter() {
                    if *uid > last_seen_uid {
                        let messages = imap_session
                            .fetch(uid.to_string(), "(FLAGS ENVELOPE BODY[HEADER])")
                            .expect("Failed to fetch");

                        for message in messages.iter() {
                            if let Some(mut code) = process_message(message) {
                                // Pick a random phrase for this code
                                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                                let index = (now.as_nanos() as usize) % phrases.len();
                                code.phrase = phrases[index].to_string();

                                // Store the latest 2FA code
                                if let Ok(mut guard) = latest_code.lock() {
                                    *guard = Some(code.clone());
                                    println!("Updated 2FA code: {} {}", code.code, code.phrase);

                                    // Save to disk
                                    if let Err(e) = save_code(&code) {
                                        eprintln!("Failed to save code to disk: {}", e);
                                    } else {
                                        println!("Saved 2FA code to code.json");
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("IDLE error: {}", e);
                break;
            }
        }
    }

    imap_session.logout().expect("Failed to logout");
}

fn process_message(fetch: &Fetch) -> Option<TwoFactorCode> {
    let allowed_services = &["ChatGPT"];

    if let Some(envelope) = fetch.envelope() {
        let subject = envelope
            .subject
            .as_ref()
            .map(|s| String::from_utf8_lossy(s))
            .unwrap_or_default();

        let from = envelope
            .from
            .as_ref()
            .and_then(|addrs| addrs.first())
            .map(|addr| {
                format!(
                    "{}@{}",
                    String::from_utf8_lossy(addr.mailbox.as_ref().map_or(&b"unknown"[..], |v| v)),
                    String::from_utf8_lossy(addr.host.as_ref().map_or(&b"unknown"[..], |v| v))
                )
            })
            .unwrap_or_else(|| "unknown".to_string());

        println!("New email from: {}", from);
        println!("Subject: {}", subject);

        // Check if this is a 2FA email using regex
        let re = Regex::new(r"Your (.+) code is (\d+)").unwrap();
        if let Some(captures) = re.captures(&subject) {
            let service = captures.get(1).map_or("", |m| m.as_str());
            let code = captures.get(2).map_or("", |m| m.as_str());

            println!("2FA code detected: {} for {}", code, service);
            println!("---");

            if !allowed_services.iter().any(|a_s| { *a_s == service }) {
                println!("Service name not among allowed_services, ignoring code");
                return None
            }

            return Some(TwoFactorCode {
                code: code.to_string(),
                service: service.to_string(),
                phrase: String::new(), // Will be set when storing
            });
        }

        println!("---");
    }
    None
}
