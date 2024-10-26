use chrono::FixedOffset;
use colored::Colorize;
use csv::WriterBuilder;
use serde::Serialize;
use std::{fs::OpenOptions, io::Write, time::Duration};

const OFFSET: Option<FixedOffset> = FixedOffset::east_opt(2 * 60 * 60);

impl DataRecord {
    fn update_time(&mut self) {
        let date_now = chrono::Utc::now()
            .with_timezone(&OFFSET.expect("unwrapping the CONST var OFFSET for local time"))
            .format("%d-%m-%Y %H:%M:%S");
        self.time = date_now.to_string();
    }
}

fn main() {
    let port_name = "/dev/ttyACM0";
    let baud_rate = 57600;
    let mut port = serialport::new(port_name, baud_rate)
        .timeout(std::time::Duration::from_millis(2000))
        .open()
        .unwrap_or_else(|e| {
            eprintln!("Failed to open \"{}\". Error: {}", port_name, e);
            ::std::process::exit(1);
        });

    let mut data_record = DataRecord {
        temperature: 0,
        humidity: 0,
        time: chrono::Utc::now().format("%d-%m-%Y %H:%M:%S").to_string(),
    };

    const BUF_LEN: usize = 12;
    let mut read_buf: [u8; BUF_LEN] = [0; BUF_LEN];
    let mut received: String;
    let mut _prev_received = "0".to_string();
    let pause = 900; // millis
    let mut point = String::new();
    let delimiter = b';';
    let mut record_buffer = vec![];

    println!("Lettura in corso ...");
    print!("\x1b[1A\r");
    loop {
        //println!("Loop");
        data_record.update_time();
        match port.read(&mut read_buf) {
            Ok(end) => {
                if read_buf.contains(&delimiter) {
                    data_record.update_time();
                    //dbg!(&read_buf);
                    received = String::from_utf8_lossy(&read_buf[..end]).to_string();
                    //println!("{}",received);
                    show_data(received, &mut point, &mut record_buffer, &mut data_record);
                } else {
                    data_record.update_time();
                    //println!("buffer doesn't contain delimiter char");
                }
            }
            Err(e) => {
                println!("Read Error: {e}")
            }
        }
        data_record.update_time();
        std::thread::sleep(Duration::from_millis(pause));

        //received = String::from_utf8(read_buf[..].to_vec()).unwrap();
        //if received != prev_received && read_buf[0] != 0 {
        //    prev_received = received
        //}
    }
}

fn show_data(
    received: String,
    point: &mut String,
    record_buffer: &mut Vec<DataRecord>,
    data_record: &mut DataRecord,
) {
    let (temp, hum): (String, String) = received
        .split(";")
        .filter_map(|x| {
            let mut split = x.split(",");

            match (split.next(), split.next()) {
                (Some(temp), Some(hum)) if temp.len() > 1 && hum.len() > 1 => Some((temp, hum)),
                (_, _) => None,
            }
        })
        .take(1)
        .unzip();
    let temp = temp.replace("t", "").parse::<i8>();
    let hum = hum.replace("h", "").parse::<u8>();

    // UPDATE DATA IF PARSING IS OK
    if let (Ok(temp), Ok(hum)) = (temp, hum) {
        data_record.humidity = hum;
        data_record.temperature = temp;
    }

    let temp_string = "Temperatura".to_string().bright_yellow().bold();
    let temp_record_str = data_record.temperature.to_string().bright_cyan();
    let cels_sym = "C°".to_string().bright_green().bold();

    let hum_string = "Umidità".to_string().bright_yellow().bold();
    let hum_record_str = data_record.humidity.to_string().bright_cyan();
    let perc_sym = "%".to_string().bright_green().bold();

    let time_string = "Time".to_string().bright_yellow().bold();
    let time_record_str = data_record.time.to_string().bright_cyan();

    //std::io::stdout().flush().expect("flush");
    println!(
        "{}: {} {}          ",
        temp_string, temp_record_str, cels_sym
    );
    println!(
        "{}: {} {}              ",
        hum_string, hum_record_str, perc_sym
    );
    println!("{}: {}                   ", time_string, time_record_str);
    println!(
        "Buffer size: {}/{}  ",
        record_buffer.len(),
        RECORD_BUFFER_IS_COMPLETE_AT_LEN
    );
    print!(
        "{} {}  ",
        "Arduino DHT11 in corso"
            .to_string()
            .italic()
            .bold()
            .bright_magenta(),
        point
    );
    print!("\x1b[1A\r\x1b[1A\x1b[1A\x1b[1A\r");
    std::io::stdout().flush().expect("flush");

    //println!("{}",format!("{}",now_date));

    if record_buffer.len() as u8 >= RECORD_BUFFER_IS_COMPLETE_AT_LEN {
        record_buffer.sort();
        record_buffer.dedup_by(|a, b| {
            matches!(
                a,
                a if a.temperature == b.temperature
                  || a.humidity == b.humidity
            )
        });

        write_data_record(
            "/home/giulio/arduino_embedded/serial_communication/temp_hum.csv",
            record_buffer,
        );

        record_buffer.clear();
    } else {
        record_buffer.push(data_record.clone());
    }

    if point.len() == 3 {
        *point = "".to_string();
    }
    *point += ".";
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DataRecord {
    #[serde(rename = "Temperatura (C°)")]
    // non serve scrivere se si usa la appen mode per scrittura
    temperature: i8,
    #[serde(rename = "Humidity (%)")]
    humidity: u8,
    #[serde(rename = "Time")]
    time: String,
}

//impl Into<Vec<u8>> for DataRecord {
//    fn into(self) -> Vec<u8> {
//        vec![self.temperature, self.humidity]
//    }
//}

const RECORD_BUFFER_IS_COMPLETE_AT_LEN: u8 = 10; // records at write
fn write_data_record(file_path: &str, records: &Vec<DataRecord>) {
    let file_path = OpenOptions::new()
        .append(true)
        .open(file_path)
        .unwrap_or_else(|_| panic!("File {} should exists", file_path));

    let mut writer = WriterBuilder::new()
        .has_headers(false)
        .delimiter(b',')
        .from_writer(file_path);

    for i in records {
        match writer.serialize(i) {
            Ok(_) => {
                dbg!(&records.to_vec());
                println!("Succesfully written");
            }
            Err(e) => println!("Error: {}", e),
        };
    }
}
