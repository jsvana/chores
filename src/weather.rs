use std::collections::HashMap;

use anyhow::{anyhow, Result};
use futures::future::join_all;
use metar::{Metar, Pressure, WeatherCondition, WeatherIntensity};
use serde::Serialize;

#[derive(Debug)]
pub struct Observation {
    pub metar: String,
    pub interpretation: Metar,
}

async fn get_metar(station: &str) -> Result<Observation> {
    const METAR_URL_BASE: &'static str =
        "https://tgftp.nws.noaa.gov/data/observations/metar/stations/";

    let metar_text = reqwest::get(format!("{}{}.TXT", METAR_URL_BASE, station.to_uppercase()))
        .await?
        .text()
        .await?;

    let mut lines: Vec<&str> = metar_text.lines().collect();
    if lines.len() < 1 {
        return Err(anyhow!("Too few lines returned in METAR for {}", station));
    }

    let metar_line = lines.remove(1);

    Ok(Observation {
        metar: metar_line.to_string(),
        interpretation: Metar::parse(metar_line)?,
    })
}

#[derive(Debug, Serialize)]
pub struct StationMetar {
    pub metar: String,
    pub pressure: Option<u16>,
    pub temperature: Option<i32>,
    pub weather: Vec<Weather>,
}

#[derive(Debug, Serialize)]
pub struct Weather {
    pub conditions: Vec<String>,
    pub intensity: String,
}

fn inhg_to_hectopascals(value: f32) -> u16 {
    (value * 33.8639).trunc() as u16
}

pub async fn build_metar_response(stations: &[String]) -> HashMap<String, StationMetar> {
    let mut futures = Vec::new();
    for station in stations {
        futures.push(get_metar(station));
    }

    let mut stations = HashMap::new();
    for result in join_all(futures).await {
        let metar = match result {
            Ok(metar) => metar,
            Err(e) => {
                tracing::warn!("Failed to fetch METAR for a station: {}", e);
                continue;
            }
        };

        let mut weather_response = Vec::new();
        for weather in metar.interpretation.weather {
            let mut conditions = Vec::new();
            for condition in weather.conditions {
                conditions.push(match condition {
                    WeatherCondition::Shallow => "shallow".to_string(),
                    WeatherCondition::Partial => "partial".to_string(),
                    WeatherCondition::Patches => "patches".to_string(),
                    WeatherCondition::LowDrifting => "low drifting".to_string(),
                    WeatherCondition::Blowing => "blowing".to_string(),
                    WeatherCondition::Showers => "showers".to_string(),
                    WeatherCondition::Thunderstorm => "thunderstorm".to_string(),
                    WeatherCondition::Freezing => "freezing".to_string(),
                    WeatherCondition::Rain => "rain".to_string(),
                    WeatherCondition::Drizzle => "drizzle".to_string(),
                    WeatherCondition::Snow => "snow".to_string(),
                    WeatherCondition::SnowGrains => "snow grains".to_string(),
                    WeatherCondition::IceCrystals => "ice crystals".to_string(),
                    WeatherCondition::IcePellets => "ice pellets".to_string(),
                    WeatherCondition::Hail => "hail".to_string(),
                    WeatherCondition::SnowPelletsOrSmallHail => {
                        "snow pellets or small hail".to_string()
                    }
                    WeatherCondition::UnknownPrecipitation => "unknown precipitation".to_string(),
                    WeatherCondition::Fog => "fog".to_string(),
                    WeatherCondition::VolcanicAsh => "volcanic ash".to_string(),
                    WeatherCondition::Mist => "mist".to_string(),
                    WeatherCondition::Haze => "haze".to_string(),
                    WeatherCondition::WidespreadDust => "widespread dust".to_string(),
                    WeatherCondition::Smoke => "smoke".to_string(),
                    WeatherCondition::Sand => "sand".to_string(),
                    WeatherCondition::Spray => "spray".to_string(),
                    WeatherCondition::Squall => "squall".to_string(),
                    WeatherCondition::Dust => "dust".to_string(),
                    WeatherCondition::Duststorm => "duststorm".to_string(),
                    WeatherCondition::Sandstorm => "sandstorm".to_string(),
                    WeatherCondition::FunnelCloud => "funnel cloud".to_string(),
                });
            }

            weather_response.push(Weather {
                conditions,
                intensity: match weather.intensity {
                    WeatherIntensity::Light => "light".to_string(),
                    WeatherIntensity::Moderate => "moderate".to_string(),
                    WeatherIntensity::Heavy => "heavy".to_string(),
                    WeatherIntensity::InVicinity => "in vicinity".to_string(),
                    WeatherIntensity::Recent => "recent".to_string(),
                },
            });
        }

        stations.insert(
            metar.interpretation.station,
            StationMetar {
                metar: metar.metar,
                pressure: match metar.interpretation.pressure {
                    metar::Data::Known(pressure) => match pressure {
                        Pressure::Hectopascals(v) => Some(v),
                        Pressure::InchesOfMercury(v) => Some(inhg_to_hectopascals(v)),
                    },
                    metar::Data::Unknown => None,
                },
                temperature: match metar.interpretation.temperature {
                    metar::Data::Known(temp) => Some(temp),
                    metar::Data::Unknown => None,
                },
                weather: weather_response,
            },
        );
    }

    stations
}
