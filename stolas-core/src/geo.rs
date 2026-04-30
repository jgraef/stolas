use std::fmt::Display;

#[derive(Clone, Copy, Debug)]
pub struct GeoCoords {
    pub latitude: f64,
    pub longitude: f64,
}

impl GeoCoords {
    pub fn format(&self) -> DelayedFormattedGeoCoords {
        let lat = prepare_coordinate(self.latitude);
        let lon = prepare_coordinate(self.longitude);

        DelayedFormattedGeoCoords { lat, lon }
    }
}

#[derive(Clone, Copy, Debug)]
struct CoordParts {
    sign: i8,
    degrees: i16,
    minutes: i8,
    seconds: i8,
}

fn prepare_coordinate(mut x: f64) -> CoordParts {
    let sign = x.signum() as i8;
    x = x.abs();
    let degrees = x.floor() as i16;
    x = x.fract() * 60.0;
    let minutes = x.floor() as i8;
    let seconds = (x.fract() * 60.0).floor() as i8;

    CoordParts {
        sign,
        degrees,
        minutes,
        seconds,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DelayedFormattedGeoCoords {
    lat: CoordParts,
    lon: CoordParts,
}

impl Display for DelayedFormattedGeoCoords {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { lat, lon } = self;

        let ns = if lat.sign >= 0 { "N" } else { "S" };
        let we = if lon.sign >= 0 { "E" } else { "W" };

        write!(
            f,
            "{}°{}'{}\"{} {}°{}'{}\"{}",
            lat.degrees, lat.minutes, lat.seconds, ns, lon.degrees, lon.minutes, lon.seconds, we
        )
    }
}
