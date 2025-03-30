use soapysdr::Device;
use std::net::UdpSocket;
use std::time::Duration;

const SAMPLE_RATE: f64 = 62_500.0;
const CENTER_FREQ: f64 = 915.0e6; // 915 MHz
const GAIN: f64 = 40.0;
const UDP_ADDR: &str = "127.0.0.1:4001"; // Replace with your receiver's IP

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize SDRPlay device
    let dev = Device::new("driver=sdrplay")?; // TODO look into this

    // Configure the SDR
    let mut rx_stream = dev.rx_stream::<f32>(&[0])?;
    dev.set_sample_rate(soapysdr::Direction::Rx, 0, SAMPLE_RATE)?;
    dev.set_frequency(soapysdr::Direction::Rx, 0, CENTER_FREQ, ())?;
    dev.set_gain(soapysdr::Direction::Rx, 0, GAIN)?;

    // Start streaming
    rx_stream.activate(None)?;

    // Create a UDP socket
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_write_timeout(Some(Duration::from_millis(100)))?;

    let mut buffer = vec![0.0f32; 4096];

    loop {
        // Read I/Q samples
        match rx_stream.read(&mut [&mut buffer], 1000000) {
            Ok(samples) => {
                let iq_bytes: &[u8] = bytemuck::cast_slice(&buffer[..samples]);
                socket.send_to(iq_bytes, UDP_ADDR)?;
            }
            Err(e) => eprintln!("Error reading SDR data: {:?}", e),
        }
    }
}
