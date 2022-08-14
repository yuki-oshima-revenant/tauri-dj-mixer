#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::BufReader;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::{fs, thread};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::Value;
use symphonia::core::meta::{self, MetadataOptions};
use symphonia::core::probe::Hint;

struct State(Mutex<Option<Arc<Sink>>>);

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetaDataVisual {
    media_type: String,
    data: Box<[u8]>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetaData {
    path: String,
    title: Option<String>,
    artist: Option<String>,
    group: Option<String>,
    album: Option<String>,
    track_number: Option<String>,
    visual: Option<MetaDataVisual>,
}

impl MetaData {
    fn new(path: &String) -> MetaData {
        MetaData {
            path: path.to_owned(),
            title: None,
            artist: None,
            group: None,
            album: None,
            track_number: None,
            visual: None,
        }
    }
}

impl fmt::Display for MetaData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let default = "".to_owned();
        let default_visual = MetaDataVisual {
            media_type: "".to_owned(),
            data: Box::new([]),
        };
        let visula_data = &self.visual.as_ref().unwrap_or(&default_visual).data;
        write!(
            f,
            "path:{}, title:{}, artist:{}, group:{}, album:{}, track_number:{}, visual:(media_type: {}, data:{},...{}), ",
            self.path,
            self.title.as_ref().unwrap_or(&default),
            self.artist.as_ref().unwrap_or(&default),
            self.group.as_ref().unwrap_or(&default),
            self.album.as_ref().unwrap_or(&default),
            self.track_number.as_ref().unwrap_or(&default),
            self.visual.as_ref().unwrap_or(&default_visual).media_type,
            format!("{},{},{},{},{}", visula_data[0], visula_data[1], visula_data[2], visula_data[3], visula_data[4]),
            format!("{},{},{},{},{}", visula_data[visula_data.len() -5], visula_data[visula_data.len() -4], visula_data[visula_data.len() -3], visula_data[visula_data.len() -2], visula_data[visula_data.len() -1]),
        )
    }
}

fn read_mp3_metadata(path: &String) -> MetaData {
    let src = std::fs::File::open(path).expect("failed to open media");
    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("mp3");
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();
    let mut probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .expect("unsupported format");
    let metadata = probed.metadata.get().unwrap();
    let tags = metadata.current().unwrap().tags();
    let mut formatted_metadata = MetaData::new(path);
    for tag in tags {
        match &tag.key[..] {
            "TIT2" => {
                if let Value::String(value) = &tag.value {
                    formatted_metadata.title = Some(value.to_string())
                }
            }
            "TALB" => {
                if let Value::String(value) = &tag.value {
                    formatted_metadata.album = Some(value.to_string())
                }
            }
            "TPE1" => {
                if let Value::String(value) = &tag.value {
                    formatted_metadata.artist = Some(value.to_string())
                }
            }
            "TPE2" => {
                if let Value::String(value) = &tag.value {
                    formatted_metadata.group = Some(value.to_string())
                }
            }
            "TRCK" => {
                if let Value::String(value) = &tag.value {
                    formatted_metadata.track_number = Some(value.to_string())
                }
            }
            _ => (),
        }
    }
    let visuals = metadata.current().unwrap().visuals();
    for visual in visuals {
        formatted_metadata.visual = Some(MetaDataVisual {
            media_type: visual.media_type.to_owned(),
            data: visual.data.to_owned(),
        });
    }
    formatted_metadata
}

#[tauri::command]
fn find_files() -> Vec<MetaData> {
    let dir = fs::read_dir("/Users/yukioshima/マイドライブ/music/Anime/eYe's").unwrap();
    let mut paths = Vec::<MetaData>::new();
    for path in dir {
        paths.push(read_mp3_metadata(
            &path.unwrap().path().to_str().unwrap().to_owned(),
        ));
    }
    paths
}

#[tauri::command]
fn play_file(path: String, state: tauri::State<State>) {
    println!("{}", path);
    let mut state_value = state.0.lock().unwrap();
    match state_value.as_ref() {
        Some(sink) => sink.stop(),
        None => (),
    };
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let file = BufReader::new(fs::File::open(path).unwrap());
        let source = Decoder::new(file).unwrap();
        let sink = Arc::new(Sink::try_new(&stream_handle).unwrap());
        sink.append(source);
        tx.send(Arc::clone(&sink)).unwrap();
        sink.sleep_until_end();
    });
    *state_value = Some(rx.recv().unwrap());
}

#[tauri::command]
fn pause_play(state: tauri::State<State>) {
    let state_value = state.0.lock().unwrap();
    let sink = state_value.as_ref().unwrap();
    if sink.is_paused() {
        sink.play();
    } else {
        sink.pause();
    }
}

fn main() {
    tauri::Builder::default()
        .manage(State(Mutex::new(Option::None)))
        .invoke_handler(tauri::generate_handler![find_files, play_file, pause_play])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test() {
        let dir = fs::read_dir("/Users/yukioshima/マイドライブ/music/Anime/eYe's").unwrap();
        for path in dir {
            let path_string = path.unwrap().path().to_str().unwrap().to_owned();
            println!("{}", read_mp3_metadata(&path_string));
        }
    }
}
