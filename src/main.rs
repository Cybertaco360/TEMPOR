use rodio::{Decoder, OutputStream, Sink};
use std::{env,fs::File,io::{stdout, BufReader, Write},path::PathBuf,sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}},thread,time::Duration,process::Command};
use crossterm::{cursor, event, terminal::{enable_raw_mode, disable_raw_mode, Clear, ClearType}, execute};
use walkdir::WalkDir;
fn get_mp3_files(folder: &str) -> Vec<PathBuf> {
    WalkDir::new(folder)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map(|ext| ext == "mp3").unwrap_or(false))
        .map(|entry| entry.into_path())
        .collect()
}

fn print_now_playing(file_path: &str) {
    execute!(
        stdout(),
        Clear(ClearType::CurrentLine),  // Clears the current line
        cursor::MoveToColumn(0)         // Moves cursor to the start of the line
    ).unwrap();
    
    print!("\r🎶 Now playing: {}", file_path);
    stdout().flush().unwrap();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <music_folder>", args[0]);
        return;
    }
    let folder_path = &args[1];
    let files = get_mp3_files(folder_path);
    if files.is_empty() {
        eprintln!("No MP3 files found in the specified folder.");
        return;
    }
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Arc::new(Mutex::new(Sink::try_new(&stream_handle).unwrap()));
    let skip_flag = Arc::new(AtomicBool::new(false));
    let mut clear_console = Command::new("clear");
    clear_console.status().expect("process failed to execute");
    enable_raw_mode().unwrap();
    println!("🎵 MP3 Player - Controls: [P]ause, [R]esume, [N]ext, [Q]uit");
    for file in files {
        execute!(stdout(), Clear(ClearType::CurrentLine)).unwrap();
        print_now_playing(&file.display().to_string());

        skip_flag.store(false, Ordering::Relaxed);

        let sink_clone = Arc::clone(&sink);
        let skip_clone = Arc::clone(&skip_flag);
        let control_thread = thread::spawn(move || {
            loop {
                if event::poll(Duration::from_millis(100)).unwrap() {
                    if let event::Event::Key(key) = event::read().unwrap() {
                        match key.code {
                            event::KeyCode::Char('p') => sink_clone.lock().unwrap().pause(),
                            event::KeyCode::Char('r') => sink_clone.lock().unwrap().play(),
                            event::KeyCode::Char('n') => {
                                skip_clone.store(true, Ordering::Relaxed);
                                sink_clone.lock().unwrap().stop();
                                break;
                            }
                            event::KeyCode::Char('q') => {
                                println!("\nGoodbye!\n");
                                disable_raw_mode().unwrap();
                                std::process::exit(0);
                            }
                            _ => {}
                        }
                    }
                }
            }
        });
        let audio_file = File::open(&file).unwrap();
        let source = Decoder::new(BufReader::new(audio_file)).unwrap();
        {
            let sink_lock = sink.lock().unwrap();
            sink_lock.append(source);
            sink_lock.play();
        }
        while !sink.lock().unwrap().empty() {
            if skip_flag.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(Duration::from_millis(500));
        }

        control_thread.join().unwrap();
    }

    disable_raw_mode().unwrap();
}
