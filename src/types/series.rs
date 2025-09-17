use serde::{Deserialize, Serialize};
use serde_json::{Value};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct Episode {
    pub id: String,
    episode_num: Value,
    pub title: String,
    pub container_extension: String,
    pub info: Value,
    custom_sid: Value,
    added: String,
    season: Value,
    direct_source: String
}

pub trait EpisodeTrait {
    fn ext(&self) -> String;
}

impl EpisodeTrait for Episode {
    fn ext(&self) -> String {
        format!(".{}", self.container_extension)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Series {
    seasons: Vec<Value>,
    info: Value,
    episodes: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SeriesVec {
    seasons: Vec<Value>,
    info: Value,
    episodes: Vec<Vec<Value>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SeriesEmpty {
    seasons: Vec<Value>,
    info: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SerEnum {
    Series(Series),
    SeriesVec(SeriesVec),
    SeriesEmpty(SeriesEmpty),
    None,
}

pub trait Episodes {
    fn get_episodes(&self) -> Vec<Episode>;
    fn series_name(&self) -> String;
}

impl Episodes for SerEnum {
    fn series_name(&self) -> String {
        match self {
            SerEnum::Series(s) => {
                s.info["name"].to_string()
            },
            SerEnum::SeriesVec(s) => {s.info["name"].to_string()},
            SerEnum::SeriesEmpty(s) => {s.info["name"].to_string()},
            SerEnum::None => "None".to_string(),
        }
    }
    
    fn get_episodes(&self) -> Vec<Episode> {
        let mut elist = Vec::new();
        match self {
            SerEnum::Series(series) => {
                let e = series.episodes.to_string();
                let e_values: HashMap<String, Value> = serde_json::from_str(&e).expect("JSON");
                let mut episodes: Vec<_> = e_values.iter().collect();
                episodes.sort_by_key(|a| a.0);
                for (_k, v) in episodes.iter() {
                    for i in v.as_array().unwrap() {
                        let ep: Episode = serde_json::from_value(i.to_owned()).unwrap();
                        elist.push(ep);
                    }
                }
                elist
            },
            SerEnum::SeriesVec(series) => {
                for e in series.episodes.iter() {
                    for e1 in e.iter() {
                        let ep: Episode = serde_json::from_value(e1.to_owned()).unwrap();
                        elist.push(ep);
                    }
                }
                elist
            },
            _ => {
                elist}
        }
    }
}
pub fn read_series(s: String) -> SerEnum{
    match serde_json::from_str::<SeriesVec>(&s) {
        Ok(x) => SerEnum::SeriesVec(x),
        Err(_) => {
            match serde_json::from_str::<Series>(&s) {
                Ok(x) => SerEnum::Series(x),
                Err(_) => {
                    match serde_json::from_str::<SeriesEmpty>(&s) {
                        Ok(x) => SerEnum::SeriesEmpty(x),
                        _ => {
                            println!("Unable to parse");
                            SerEnum::None
                        },
                    }
                }
            }
        }
    }
}
