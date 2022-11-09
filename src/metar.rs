use std::collections::HashMap;

use metar::Metar;

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
    pub temperature: Option<i32>,
}

pub fn build_metar_response(stations: &[String]) -> HashMap<String, StationMetar> {
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

        let weather_response = Vec::new();
        for weather in metar.interpretation.weather {
            weather_response.push(Weather {
                intensity: match weather.intensity {
                    WeatherIntensity::Light => "light",
                    WeatherIntensity::Moderate => "moderate",
                    WeatherIntensity::Heavy => "heavy",
                    WeatherIntensity::InVicinity => "in vicinity",
                    WeatherIntensity::Recent => "recent",
                },
            });
        }

        stations.insert(
            metar.interpretation.station,
            StationMetar {
                metar: metar.metar,
                temperature: match metar.interpretation.temperature {
                    metar::Data::Known(temp) => Some(temp),
                    metar::Data::Unknown => None,
                },
            },
        );
    }
}
