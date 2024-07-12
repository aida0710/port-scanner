use pnet::datalink;
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::tcp::{MutableTcpPacket, TcpFlags};
use pnet::packet::ipv4::MutableIpv4Packet;
use pnet::transport::{transport_channel, TransportChannelType::Layer4, TransportReceiver, TransportSender, tcp_packet_iter};
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;
use std::thread;
use std::env;

fn get_local_ipv4() -> Option<Ipv4Addr> {
    for interface in datalink::interfaces() {
        for ip in interface.ips {
            if let IpAddr::V4(ipv4) = ip.ip() {
                if !ipv4.is_loopback() {
                    return Some(ipv4);
                }
            }
        }
    }
    None
}

fn scan_port(
    packet_sender: &mut TransportSender,
    packet_receiver: &mut TransportReceiver,
    source_ip: Ipv4Addr,
    destination_ip: Ipv4Addr,
    port: u16
) {
    let source_port = 54321;
    // SYNパケットの作成
    let mut ipv4_buffer = [0u8; 20];
    let mut ipv4_packet = MutableIpv4Packet::new(&mut ipv4_buffer).unwrap();
    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length(5);
    ipv4_packet.set_total_length(40);
    ipv4_packet.set_ttl(64);
    ipv4_packet.set_next_level_protocol(IpNextHeaderProtocols::Tcp);
    ipv4_packet.set_source(source_ip);
    ipv4_packet.set_destination(destination_ip);

    let mut tcp_buffer = [0u8; 20];
    let mut tcp_packet = MutableTcpPacket::new(&mut tcp_buffer).unwrap();
    tcp_packet.set_source(source_port);
    tcp_packet.set_destination(port);
    tcp_packet.set_sequence(0);
    tcp_packet.set_flags(TcpFlags::SYN);
    tcp_packet.set_window(64240);
    tcp_packet.set_data_offset(5);

    // パケットの送信
    match packet_sender.send_to(tcp_packet, IpAddr::V4(destination_ip)) {
        Ok(_) => (),
        Err(e) => eprintln!("ポート {}へのSYNパケット送信に失敗しました: {}", port, e),
    }

    // 応答の待機
    let timeout = Duration::from_secs(5);
    let start_time = std::time::Instant::now();

    let mut packet_iterator = tcp_packet_iter(packet_receiver);
    while start_time.elapsed() < timeout {
        match packet_iterator.next() {
            Ok((tcp, _)) => {
                if tcp.get_destination() == source_port {
                    let flags = tcp.get_flags();
                    if flags & TcpFlags::SYN != 0 && flags & TcpFlags::ACK != 0 {
                        println!("ポート {} は開いています（SYN-ACK受信）", port);
                        return;
                    } else if flags & TcpFlags::RST != 0 {
                        println!("ポート {} は閉じています（RST受信）", port);
                        return;
                    }
                }
            }
            Err(e) => eprintln!("パケット読み取り中にエラーが発生しました: {}", e),
        }
        thread::sleep(Duration::from_millis(100));
    }
    println!("ポート {} はフィルタリングされているか、ホストがダウンしています（応答なし）", port);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("使用方法: {} <送信元IP> <送信先IP> <開始ポート-終了ポート>", args[0]);
        std::process::exit(1);
    }

    let source_ip: Ipv4Addr = if args[1] == "0.0.0.0" {
        match get_local_ipv4() {
            Some(ip) => {
                println!("ローカルIPアドレスを使用します: {}", ip);
                ip
            },
            None => {
                eprintln!("ローカルIPアドレスを取得できませんでした。");
                std::process::exit(1);
            }
        }
    } else {
        args[1].parse().expect("無効な送信元IPアドレスです")
    };

    let destination_ip: Ipv4Addr = args[2].parse().expect("無効な送信先IPアドレスです");
    let port_range: Vec<&str> = args[3].split('-').collect();
    let start_port: u16 = port_range[0].parse().expect("無効な開始ポートです");
    let end_port: u16 = port_range[1].parse().expect("無効な終了ポートです");

    if start_port == 0 || end_port == 0 {
        eprintln!("エラー: ポート番号は1以上である必要があります。");
        std::process::exit(1);
    }

    if start_port > end_port {
        eprintln!("エラー: 開始ポート（{}）が終了ポート（{}）より大きいです。", start_port, end_port);
        eprintln!("開始ポートは終了ポート以下である必要があります。");
        std::process::exit(1);
    }

    let protocol = Layer4(pnet::transport::TransportProtocol::Ipv4(IpNextHeaderProtocols::Tcp));
    let (mut packet_sender, mut packet_receiver) = match transport_channel(4096, protocol) {
        Ok((sender, receiver)) => (sender, receiver),
        Err(e) => panic!("トランスポートチャンネルの作成中にエラーが発生しました: {}", e),
    };

    println!("{}のポート{}から{}をスキャンしています...", destination_ip, start_port, end_port);

    for port in start_port..=end_port {
        scan_port(&mut packet_sender, &mut packet_receiver, source_ip, destination_ip, port);
    }

    println!("スキャンが完了しました");
}