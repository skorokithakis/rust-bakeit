extern crate curl;
extern crate rustc_serialize;
extern crate docopt;
extern crate url;
extern crate ini;

use ini::Ini;
use url::Url;
use curl::easy::{Easy, List};
use docopt::Docopt;
use std::env::home_dir;
use std::str;
use std::fs::File;
use std::io::{Read, stdin};
use rustc_serialize::json::{Json, Object};


const VERSION: &'static str = env!("CARGO_PKG_VERSION");

const USAGE: &'static str = "
Bakeit.

Usage:
  bakeit [options] [<filename>]

Options:
  -t --title=<title>      The title of the paste.
  -l --lang=<lang>        The language highlighter to use.
  -d --duration=<mins>    The duration the paste should live for [default: 60].
  -v --max-views=<views>  How many times the paste can be viewed before it expires [default: 0].
  -b --open-browser       Automatically open a browser window when done.

  -h --help               Show this screen.
  -V --version            Show version.
";

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_filename: Option<String>,
    flag_title: Option<String>,
    flag_lang: Option<String>,
    flag_duration: i32,
    flag_max_views: i32,
    flag_open_browser: bool,
    flag_version: bool,
}

#[derive(Default)]
struct Config {
    api_key: String,
}

macro_rules! die{($e:expr) => {println!("{}", $e); std::process::exit(1)}}

fn upload(input: String,
          api_key: String,
          title: Option<String>,
          language: Option<String>,
          duration: i32,
          max_views: i32)
          -> Result<Object, String> {

    let mut data = input.as_bytes();
    let mut response = Vec::new();

    let mut url = Url::parse("https://www.pastery.net/api/paste/").unwrap();
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("api_key", &api_key);
        if let Some(title) = title {
            qp.append_pair("title", &title);
        }
        if let Some(language) = language {
            qp.append_pair("language", &language);
        }
        if duration > 0 {
            qp.append_pair("duration", &duration.to_string());
        }
        if max_views > 0 {
            qp.append_pair("max_views", &max_views.to_string());
        }
    };

    let mut easy = Easy::new();
    easy.url(url.as_str())
        .unwrap();

    let mut headers = List::new();
    headers.append("User-Agent: 'Mozilla/5.0 (Rust) bakeit library").unwrap();

    easy.http_headers(headers).unwrap();
    easy.post(true).unwrap();
    easy.post_field_size(data.len() as u64).unwrap();

    {
        let mut transfer = easy.transfer();
        transfer.read_function(|buf| Ok(data.read(buf).unwrap_or(0))).unwrap();
        transfer.write_function(|new_data| {
                response.extend_from_slice(new_data);
                Ok(new_data.len())
            })
            .unwrap();
        transfer.perform().unwrap();
    }

    let response = &str::from_utf8(&response).unwrap().to_string();

    match easy.response_code().unwrap() {
        500...600 => Err(String::from("There was a server error, please try again later.")),
        413 => {
            Err(String::from("The chosen file was rejected by the server because it was too \
                              large, please try a smaller file."))
        }
        422 => Err(parse_response(response).get("error_msg").unwrap().to_string()),
        _ => Ok(parse_response(response)),
    }

}

fn parse_response(response: &String) -> Object {
    let json = Json::from_str(&response).unwrap();
    json.as_object().unwrap().clone()
}

fn read_config() -> Config {
    let mut config = Config::default();
    let mut conf_path = home_dir().unwrap();
    conf_path.push(".config/bakeit.cfg");

    let conf = Ini::load_from_file(conf_path.to_str().unwrap()).unwrap_or_else(|_| {
        die!("Config file not found. Make sure you have a config file at ~/.config/bakeit.cfg \
              with a [pastery] section containing your Pastery API key, which you can get from \
              your https://www.pastery.net account page.");
    });

    let section = conf.section(Some("pastery".to_owned())).unwrap_or_else(|| {
        die!("[pastery] section not found. Please add a [pastery] section to \
        the ~/.config/bakeit.cfg file and try again.");
    });
    config.api_key = section.get("api_key")
        .unwrap_or_else(|| {
            die!("No api_key entry found. Please add an api_key entry to the [pastery] section \
                  with your API key in it. You can find the latter on your account page on \
                  https://www.pastery.net.");
        })
        .clone();

    config
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    let mut input = String::new();

    if args.flag_version {
        println!("bakeit {}", VERSION);
        std::process::exit(0);
    }

    match args.arg_filename {
        None => {
            println!("Type your paste and press Ctrl+D to upload.");
            stdin().read_to_string(&mut input).unwrap_or_else(|e| {
                die!(e);
            });
        }
        Some(filename) => {
            let mut file = File::open(filename).unwrap_or_else(|e| {
                die!(e.to_string());
            });
            file.read_to_string(&mut input).unwrap_or_else(|e| {
                die!(e);
            });
        }
    }

    let config = read_config();

    println!("Uploading to Pastery...");
    let response = upload(input,
                          config.api_key,
                          args.flag_title,
                          args.flag_lang,
                          args.flag_duration,
                          args.flag_max_views)
        .unwrap_or_else(|e| {
            die!(e);
        });

    println!("Paste URL: {}",
             response.get("url").expect("URL not found in response.").as_string().unwrap());

    if args.flag_open_browser {
    }
}
