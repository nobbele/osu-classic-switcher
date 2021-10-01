use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream},
};

pub const TARGET_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 0, 108);
pub const TARGET_PORT: u16 = 13382;
pub const HOSTS_PATH: &'static str = r"C:\Windows\System32\drivers\etc\hosts";
pub const COMMENT: &'static str = "# Added by osu!classic";

fn get_target_hosts() -> HashMap<&'static str, Ipv4Addr> {
    let mut target_hosts = HashMap::new();
    target_hosts.insert("osu.ppy.sh", Ipv4Addr::new(127, 0, 0, 1));
    target_hosts.insert("a.ppy.sh", Ipv4Addr::new(127, 0, 0, 1));
    target_hosts
}

fn main() {
    set_hosts();
    ctrlc::set_handler(|| {
        clear_hosts();
        std::process::exit(0);
    })
    .unwrap();
    run_proxy();
}

fn clear_hosts() {
    let target_hosts = get_target_hosts();
    let hosts = std::fs::read_to_string(HOSTS_PATH).unwrap();

    let filtered_hosts = hosts
        .lines()
        .filter(|&line| {
            if line == COMMENT {
                return false;
            }
            if let Some((ip, host)) = line.split_once(' ') {
                let _ip = ip.trim();
                let host = host.trim();
                !target_hosts.contains_key(host)
            } else {
                true
            }
        })
        .collect::<Vec<_>>()
        .join("\r\n");
    std::fs::write(HOSTS_PATH, filtered_hosts).unwrap();
}

fn set_hosts() {
    clear_hosts();

    let target_hosts = get_target_hosts();
    let hosts = std::fs::read_to_string(HOSTS_PATH).unwrap();

    let new_hosts = hosts
        .lines()
        .chain(std::iter::once(""))
        .chain(std::iter::once(COMMENT))
        .map(|s| s.to_owned())
        .chain(
            target_hosts
                .iter()
                .map(|(host, ip)| format!("{} {}", ip, host)),
        )
        .collect::<Vec<_>>()
        .join("\r\n");

    std::fs::write(HOSTS_PATH, new_hosts).unwrap();
}

fn run_proxy() {
    let local_socket = SocketAddrV4::new(Ipv4Addr::LOCALHOST, TARGET_PORT);
    let target_socket = SocketAddrV4::new(TARGET_IP, TARGET_PORT);

    println!("Starting proxy ({} -> {})", local_socket, target_socket);
    let listener = TcpListener::bind(local_socket).unwrap();

    for incoming_stream in listener.incoming() {
        let incoming_stream = if let Ok(s) = incoming_stream {
            s
        } else {
            continue;
        };
        println!("Starting new connection!");
        std::thread::Builder::new()
            .name(format!(
                "handle thread for {}",
                incoming_stream.peer_addr().unwrap()
            ))
            .spawn(move || {
                let mut stream_tx = incoming_stream;
                let mut stream_rx = stream_tx.try_clone().unwrap();
                let mut target_stream_tx = TcpStream::connect(target_socket).unwrap();
                let mut target_stream_rx = target_stream_tx.try_clone().unwrap();

                let threads = [
                    std::thread::Builder::new()
                        .name(format!(
                            "stream_rx -> target_stream_tx ({})",
                            stream_rx.peer_addr().unwrap()
                        ))
                        .spawn(move || {
                            std::io::copy(&mut stream_rx, &mut target_stream_tx).unwrap();
                        })
                        .unwrap(),
                    std::thread::Builder::new()
                        .name(format!(
                            "target_stream_rx -> stream_tx ({})",
                            stream_tx.peer_addr().unwrap()
                        ))
                        .spawn(move || {
                            std::io::copy(&mut target_stream_rx, &mut stream_tx).unwrap();
                        })
                        .unwrap(),
                ];
                for thread in threads {
                    thread.join().unwrap();
                }
            })
            .unwrap();
    }
}
