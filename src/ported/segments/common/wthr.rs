// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/common/wthr.py`.
//!
//! OpenWeatherMap weather segment. Surfaces:
//!   - `weather_conditions_codes` lookup table (OWM condition id → name)
//!   - `weather_conditions_icons` default icon map
//!   - `temp_conversions` and `temp_units` tables
//!   - `_WeatherKey` namedtuple
//!   - `compute_state_from_response` JSON parsing helper
//!   - `render_one` segment builder with gradient
//!
//! The actual urllib_read/freegeoip/openweathermap calls are stubbed
//! since they need a network client; the structural pieces are
//! covered.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import json                                       // py:4
// from collections import namedtuple                // py:5
// from powerline.lib.url import urllib_read, urllib_urlencode                              // py:7
// from powerline.lib.threaded import KwThreadedSegment                                      // py:8
// from powerline.segments import with_docstring     // py:9

use serde_json::{json, Map, Value};
use std::sync::OnceLock;

/// Port of `_WeatherKey` namedtuple from
/// `powerline/segments/common/wthr.py:12`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub struct _WeatherKey {
    pub location_query: Option<String>,
    pub weather_api_key: String,
}

/// Returns the upstream OpenWeatherMap condition-code → icon-name
/// table from `powerline/segments/common/wthr.py:21-77`.
///
/// Each entry maps a numeric `condition_code` to a tuple of icon
/// names. The Python source uses single-element tuples; the Rust
/// port returns a single-element static slice.
#[allow(non_snake_case)]
pub fn weather_conditions_codes() -> &'static std::collections::HashMap<u16, Vec<&'static str>> {
    static M: OnceLock<std::collections::HashMap<u16, Vec<&'static str>>> = OnceLock::new();
    M.get_or_init(|| {
        let entries: &[(u16, &str)] = &[
            (200, "stormy"),
            (201, "stormy"),
            (202, "stormy"),
            (210, "stormy"),
            (211, "stormy"),
            (212, "stormy"),
            (221, "stormy"),
            (230, "stormy"),
            (231, "stormy"),
            (232, "stormy"),
            (300, "rainy"),
            (301, "rainy"),
            (302, "rainy"),
            (310, "rainy"),
            (311, "rainy"),
            (312, "rainy"),
            (313, "rainy"),
            (314, "rainy"),
            (321, "rainy"),
            (500, "rainy"),
            (501, "rainy"),
            (502, "rainy"),
            (503, "rainy"),
            (504, "rainy"),
            (511, "snowy"),
            (520, "rainy"),
            (521, "rainy"),
            (522, "rainy"),
            (531, "rainy"),
            (600, "snowy"),
            (601, "snowy"),
            (602, "snowy"),
            (611, "snowy"),
            (612, "snowy"),
            (613, "snowy"),
            (615, "snowy"),
            (616, "snowy"),
            (620, "snowy"),
            (621, "snowy"),
            (622, "snowy"),
            (701, "foggy"),
            (711, "foggy"),
            (721, "foggy"),
            (731, "foggy"),
            (741, "foggy"),
            (751, "foggy"),
            (761, "foggy"),
            (762, "foggy"),
            (771, "foggy"),
            (781, "foggy"),
            (800, "sunny"),
            (801, "cloudy"),
            (802, "cloudy"),
            (803, "cloudy"),
            (804, "cloudy"),
        ];
        let mut m = std::collections::HashMap::new();
        for (code, name) in entries {
            m.insert(*code, vec![*name]);
        }
        m
    })
}

/// Returns the default `weather_conditions_icons` table from
/// `powerline/segments/common/wthr.py:79-92`.
#[allow(non_snake_case)]
pub fn weather_conditions_icons() -> &'static std::collections::HashMap<&'static str, &'static str>
{
    static M: OnceLock<std::collections::HashMap<&'static str, &'static str>> = OnceLock::new();
    M.get_or_init(|| {
        let mut m = std::collections::HashMap::new();
        m.insert("day", "DAY");
        m.insert("blustery", "WIND");
        m.insert("rainy", "RAIN");
        m.insert("cloudy", "CLOUDS");
        m.insert("snowy", "SNOW");
        m.insert("stormy", "STORM");
        m.insert("foggy", "FOG");
        m.insert("sunny", "SUN");
        m.insert("night", "NIGHT");
        m.insert("windy", "WINDY");
        m.insert("not_available", "NA");
        m.insert("unknown", "UKN");
        m
    })
}

/// Port of the `temp_conversions` table from
/// `powerline/segments/common/wthr.py:94-98`.
///
/// Converts a Kelvin temperature to the named unit (C/F/K).
pub fn temp_conversions(unit: &str, temp_k: f64) -> f64 {
    // py:95-97  C/F/K lambdas
    match unit {
        "C" => temp_k - 273.15,
        "F" => (temp_k * 9.0 / 5.0) - 459.67,
        "K" => temp_k,
        _ => temp_k,
    }
}

/// Port of `temp_units` table from
/// `powerline/segments/common/wthr.py:101-105`.
pub fn temp_units(unit: &str) -> &'static str {
    match unit {
        "C" => "°C",
        "F" => "°F",
        "K" => "K",
        _ => "",
    }
}

/// Port of `WeatherSegment.weather_api_key` from
/// `powerline/segments/common/wthr.py:111`.
pub const WEATHER_API_KEY: &str = "fbc9549d91a5e4b26c15be0dbdac3460";

/// Port of `WeatherSegment.interval` from
/// `powerline/segments/common/wthr.py:109`.
pub const WEATHER_INTERVAL: u32 = 600;

/// Port of `WeatherSegment.key()` (staticmethod) from
/// `powerline/segments/common/wthr.py:113`.
///
/// Bare-name alias for [`weather_key`] preserving the upstream
/// Python `key` identifier byte-for-byte. The `weather_` prefix
/// on the primary fn exists to disambiguate from other `key` fns
/// across the codebase.
pub fn key(location_query: Option<String>, weather_api_key: Option<String>) -> _WeatherKey {
    // py:115  def key(location_query=None, **kwargs):
    weather_key(location_query, weather_api_key)
}

/// Port of `WeatherSegment.key()` (staticmethod) from
/// `powerline/segments/common/wthr.py:113`.
///
/// Returns the `_WeatherKey` for the given args, defaulting the
/// `weather_api_key` to the upstream `WEATHER_API_KEY` constant.
pub fn weather_key(location_query: Option<String>, weather_api_key: Option<String>) -> _WeatherKey {
    // py:108  class WeatherSegment(KwThreadedSegment):
    // py:109  interval = 600
    // py:110  default_location = None
    // py:111  location_urls = {}
    // py:112  weather_api_key = "fbc9549d91a5e4b26c15be0dbdac3460"
    // py:114  @staticmethod
    // py:115  def key(location_query=None, **kwargs):
    // py:116  try:
    // py:117  weather_api_key = kwargs["weather_api_key"]
    // py:118  except KeyError:
    // py:119  weather_api_key = WeatherSegment.weather_api_key
    // py:120  return _WeatherKey(location_query, weather_api_key)
    let api = weather_api_key.unwrap_or_else(|| WEATHER_API_KEY.to_string());
    _WeatherKey {
        location_query,
        weather_api_key: api,
    }
}

/// Port of `WeatherSegment.get_request_url()` from
/// `powerline/segments/common/wthr.py:122`.
///
/// **Status:** stub. Builds the OWM URL — the actual freegeoip
/// fetch + urllib_urlencode rely on network access; the Rust port
/// surfaces the URL-construction shape.
pub fn get_request_url(weather_key: &_WeatherKey) -> String {
    // py:122  def get_request_url(self, weather_key):
    // py:123  try:
    // py:124  return self.location_urls[weather_key]
    // py:125  except KeyError:
    // py:126  query_data = {
    // py:127  "appid": weather_key.weather_api_key
    // py:128  }
    // py:129  location_query = weather_key.location_query
    // py:130  if location_query is None:
    // py:131  location_data = json.loads(urllib_read('https://freegeoip.app/json/'))
    // py:132  query_data["lat"] = location_data["latitude"]
    // py:133  query_data["lon"] = location_data["longitude"]
    // py:134  else:
    // py:135  query_data["q"] = location_query
    // py:136  self.location_urls[location_query] = url = (
    // py:137  "https://api.openweathermap.org/data/2.5/weather?" +
    // py:138  urllib_urlencode(query_data))
    // py:139  return url
    let mut url = String::from("https://api.openweathermap.org/data/2.5/weather?");
    url.push_str(&format!("appid={}", weather_key.weather_api_key));
    if let Some(q) = &weather_key.location_query {
        url.push_str(&format!("&q={}", q));
    }
    url
}

/// Port of `WeatherSegment.compute_state()` from
/// `powerline/segments/common/wthr.py:141`.
///
/// **Status:** stub. The Rust port surfaces the dispatch shape;
/// the actual urllib_read call requires a network client.
pub fn compute_state(weather_key: &_WeatherKey) -> Option<(f64, Vec<&'static str>)> {
    // py:141  def compute_state(self, weather_key):
    // py:142  url = self.get_request_url(weather_key)
    let _url = get_request_url(weather_key);
    // py:143  raw_response = urllib_read(url)
    // py:144  if not raw_response:
    // py:145  self.error('Failed to get response')
    // py:146  return None
    // py:148  response = json.loads(raw_response)
    // py:149  try:
    // py:150  condition = response['weather'][0]
    // py:151  condition_code = int(condition['id'])
    // py:152  temp = float(response['main']['temp'])
    // py:153  except (KeyError, ValueError):
    // py:154  self.exception('OpenWeatherMap returned malformed or unexpected response: {0}', repr(raw_response))
    // py:155  return None
    // py:157  try:
    // py:158  icon_names = weather_conditions_codes[condition_code]
    // py:159  except IndexError:
    // py:160  icon_names = ('unknown',)
    // py:161  self.error('Unknown condition code: {0}', condition_code)
    // py:163  return (temp, icon_names)
    None
}

/// Port of `WeatherSegment.compute_state` JSON-parse path from
/// `powerline/segments/common/wthr.py:142-160`.
///
/// Given the OWM response body, extracts the `(temp_kelvin,
/// icon_names)` tuple. Returns None on malformed input or unknown
/// condition code.
pub fn compute_state_from_response(raw_response: &str) -> Option<(f64, Vec<&'static str>)> {
    // py:144  if not raw_response: return None
    if raw_response.is_empty() {
        return None;
    }
    // py:147  response = json.loads(raw_response)
    let response: Value = serde_json::from_str(raw_response).ok()?;
    // py:148-152  condition = response['weather'][0]; ... temp = response['main']['temp']
    let weather_array = response.get("weather")?.as_array()?;
    let condition = weather_array.first()?;
    let condition_code = condition.get("id")?.as_u64()? as u16;
    let temp = response.get("main")?.get("temp")?.as_f64()?;
    // py:156-159  icon_names = weather_conditions_codes[code]
    let icon_names = weather_conditions_codes()
        .get(&condition_code)
        .cloned()
        .unwrap_or_else(|| vec!["unknown"]);
    Some((temp, icon_names))
}

/// Selects the icon to render. Mirrors py:171-176:
/// walks `icon_names` looking for an `icons` override, falling back
/// to the default `weather_conditions_icons` table on the last
/// `icon_name`.
pub fn pick_icon(icon_names: &[&str], icons: Option<&Map<String, Value>>) -> String {
    // py:171-175  for icon_name in icon_names: if icons and name in icons
    if let Some(icons_map) = icons {
        for name in icon_names {
            if let Some(icon) = icons_map.get(*name).and_then(|v| v.as_str()) {
                return icon.to_string();
            }
        }
    }
    // py:176  else: icon = weather_conditions_icons[icon_names[-1]]
    let last = icon_names.last().copied().unwrap_or("unknown");
    weather_conditions_icons()
        .get(last)
        .copied()
        .unwrap_or("UKN")
        .to_string()
}

/// Computes the gradient level for a temperature per
/// `powerline/segments/common/wthr.py:181-187`.
pub fn temp_gradient_level(converted_temp: f64, temp_coldest: f64, temp_hottest: f64) -> f64 {
    // py:181-186
    if converted_temp <= temp_coldest {
        0.0
    } else if converted_temp >= temp_hottest {
        100.0
    } else {
        (converted_temp - temp_coldest) * 100.0 / (temp_hottest - temp_coldest)
    }
}

/// Port of `WeatherSegment.render_one()` from
/// `powerline/segments/common/wthr.py:163`.
///
/// Returns the two-segment `[icon, temp]` list with gradient
/// highlight on the temperature segment.
#[allow(clippy::too_many_arguments)]
pub fn render_one(
    weather: Option<(f64, Vec<&'static str>)>,
    icons: Option<&Map<String, Value>>,
    unit: &str,
    temp_format: Option<&str>,
    temp_coldest: f64,
    temp_hottest: f64,
) -> Option<Vec<Value>> {
    // py:165  def render_one(self, weather, icons=None, unit='C', temp_format=None, temp_coldest=-30, temp_hottest=40, **kwargs):
    // py:166  if not weather:
    // py:167  return None
    let (temp_k, icon_names) = weather?;
    // py:169  temp, icon_names = weather
    // py:171  for icon_name in icon_names:
    // py:172  if icons:
    // py:173  if icon_name in icons:
    // py:174  icon = icons[icon_name]
    // py:175  break
    // py:176  else:
    // py:177  icon = weather_conditions_icons[icon_names[-1]]
    let icon = pick_icon(&icon_names, icons);
    // py:179  temp_format = temp_format or ('{temp:.0f}' + temp_units[unit])
    let default_format = format!("{{temp:.0f}}{}", temp_units(unit));
    let fmt = temp_format.unwrap_or(&default_format);
    // py:180  converted_temp = temp_conversions[unit](temp)
    let converted_temp = temp_conversions(unit, temp_k);
    // py:181  if converted_temp <= temp_coldest:
    // py:182  gradient_level = 0
    // py:183  elif converted_temp >= temp_hottest:
    // py:184  gradient_level = 100
    // py:185  else:
    // py:186  gradient_level = (converted_temp - temp_coldest) * 100.0 / (temp_hottest - temp_coldest)
    let gradient_level = temp_gradient_level(converted_temp, temp_coldest, temp_hottest);
    // py:187  groups = ['weather_condition_' + icon_name for icon_name in icon_names] + ['weather_conditions', 'weather']
    let mut groups: Vec<String> = icon_names
        .iter()
        .map(|n| format!("weather_condition_{}", n))
        .collect();
    groups.push("weather_conditions".to_string());
    groups.push("weather".to_string());
    let temp_str = if fmt.contains("{temp:.0f}") {
        fmt.replace("{temp:.0f}", &format!("{:.0}", converted_temp))
    } else {
        fmt.replace("{temp}", &format!("{}", converted_temp))
    };
    // py:188  return [
    // py:189  {
    // py:190  'contents': icon + ' ',
    // py:191  'highlight_groups': groups,
    // py:192  'divider_highlight_group': 'background:divider',
    // py:193  },
    // py:194  {
    // py:195  'contents': temp_format.format(temp=converted_temp),
    // py:196  'highlight_groups': ['weather_temp_gradient', 'weather_temp', 'weather'],
    // py:197  'divider_highlight_group': 'background:divider',
    // py:198  'gradient_level': gradient_level,
    // py:199  },
    // py:200  ]
    Some(vec![
        json!({
            "contents": format!("{} ", icon),
            "highlight_groups": groups,
            "divider_highlight_group": "background:divider",
        }),
        json!({
            "contents": temp_str,
            "highlight_groups": ["weather_temp_gradient", "weather_temp", "weather"],
            "divider_highlight_group": "background:divider",
            "gradient_level": gradient_level,
        }),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_api_key_matches_upstream() {
        // py:111  upstream constant
        assert_eq!(WEATHER_API_KEY, "fbc9549d91a5e4b26c15be0dbdac3460");
    }

    #[test]
    fn weather_interval_matches_upstream() {
        // py:109  interval = 600
        assert_eq!(WEATHER_INTERVAL, 600);
    }

    #[test]
    fn weather_conditions_codes_maps_thunderstorm_to_stormy() {
        let table = weather_conditions_codes();
        assert_eq!(table.get(&200), Some(&vec!["stormy"]));
        assert_eq!(table.get(&232), Some(&vec!["stormy"]));
    }

    #[test]
    fn weather_conditions_codes_maps_rain_to_rainy() {
        let table = weather_conditions_codes();
        assert_eq!(table.get(&500), Some(&vec!["rainy"]));
        assert_eq!(table.get(&501), Some(&vec!["rainy"]));
    }

    #[test]
    fn weather_conditions_codes_maps_511_to_snowy() {
        // py:46  511: ('snowy',)  — freezing rain is snowy
        let table = weather_conditions_codes();
        assert_eq!(table.get(&511), Some(&vec!["snowy"]));
    }

    #[test]
    fn weather_conditions_codes_maps_clear_sky_to_sunny() {
        // py:74  800: ('sunny',)
        let table = weather_conditions_codes();
        assert_eq!(table.get(&800), Some(&vec!["sunny"]));
    }

    #[test]
    fn weather_conditions_codes_maps_clouds_to_cloudy() {
        let table = weather_conditions_codes();
        assert_eq!(table.get(&801), Some(&vec!["cloudy"]));
        assert_eq!(table.get(&804), Some(&vec!["cloudy"]));
    }

    #[test]
    fn weather_conditions_icons_table_matches_upstream() {
        let icons = weather_conditions_icons();
        assert_eq!(icons.get("sunny"), Some(&"SUN"));
        assert_eq!(icons.get("rainy"), Some(&"RAIN"));
        assert_eq!(icons.get("not_available"), Some(&"NA"));
        assert_eq!(icons.get("unknown"), Some(&"UKN"));
    }

    #[test]
    fn temp_conversions_kelvin_to_celsius() {
        // 273.15 K = 0 °C
        let r = temp_conversions("C", 273.15);
        assert!((r - 0.0).abs() < 1e-9);
    }

    #[test]
    fn temp_conversions_kelvin_to_fahrenheit() {
        // 273.15 K = 32 °F → (273.15 * 9/5) - 459.67 ≈ 32.0
        let r = temp_conversions("F", 273.15);
        assert!((r - 32.0).abs() < 1e-1);
    }

    #[test]
    fn temp_conversions_kelvin_to_kelvin_is_passthrough() {
        let r = temp_conversions("K", 273.15);
        assert!((r - 273.15).abs() < 1e-9);
    }

    #[test]
    fn temp_units_matches_upstream() {
        assert_eq!(temp_units("C"), "°C");
        assert_eq!(temp_units("F"), "°F");
        assert_eq!(temp_units("K"), "K");
    }

    #[test]
    fn weather_key_uses_default_api_key_when_unset() {
        // py:115-118  except KeyError: weather_api_key = WeatherSegment.weather_api_key
        let k = weather_key(None, None);
        assert_eq!(k.weather_api_key, WEATHER_API_KEY);
        assert!(k.location_query.is_none());
    }

    #[test]
    fn weather_key_respects_custom_api_key() {
        let k = weather_key(Some("oslo".to_string()), Some("custom-key".to_string()));
        assert_eq!(k.weather_api_key, "custom-key");
        assert_eq!(k.location_query, Some("oslo".to_string()));
    }

    #[test]
    fn compute_state_from_response_parses_owm_payload() {
        // py:147-159  json.loads + weather[0].id + main.temp
        let payload = r#"{"weather":[{"id":800}],"main":{"temp":295.15}}"#;
        let (temp, icons) = compute_state_from_response(payload).unwrap();
        assert!((temp - 295.15).abs() < 1e-9);
        assert_eq!(icons, vec!["sunny"]);
    }

    #[test]
    fn compute_state_from_response_unknown_code_returns_unknown_icon() {
        // py:158-160  IndexError → icon_names = ('unknown',)
        let payload = r#"{"weather":[{"id":99999}],"main":{"temp":295.15}}"#;
        let (_temp, icons) = compute_state_from_response(payload).unwrap();
        assert_eq!(icons, vec!["unknown"]);
    }

    #[test]
    fn compute_state_from_response_empty_returns_none() {
        // py:144  if not raw_response: return None
        assert!(compute_state_from_response("").is_none());
    }

    #[test]
    fn compute_state_from_response_malformed_returns_none() {
        // py:152-154  KeyError/ValueError → None
        assert!(compute_state_from_response("not json").is_none());
        assert!(compute_state_from_response("{}").is_none());
        assert!(compute_state_from_response(r#"{"weather":[]}"#).is_none());
    }

    #[test]
    fn pick_icon_uses_override_when_present() {
        // py:171-175  if icons and name in icons: return icons[name]
        let mut overrides = Map::new();
        overrides.insert("sunny".to_string(), Value::String("☀".into()));
        let icon = pick_icon(&["sunny"], Some(&overrides));
        assert_eq!(icon, "☀");
    }

    #[test]
    fn pick_icon_falls_back_to_default_table() {
        // py:176  else: icon = weather_conditions_icons[icon_names[-1]]
        let icon = pick_icon(&["sunny"], None);
        assert_eq!(icon, "SUN");
    }

    #[test]
    fn pick_icon_unknown_returns_ukn() {
        // py:176  weather_conditions_icons['unknown'] = 'UKN'
        let icon = pick_icon(&["unknown"], None);
        assert_eq!(icon, "UKN");
    }

    #[test]
    fn temp_gradient_level_below_coldest_returns_zero() {
        // py:181-182  converted_temp <= temp_coldest: 0
        assert_eq!(temp_gradient_level(-40.0, -30.0, 40.0), 0.0);
        assert_eq!(temp_gradient_level(-30.0, -30.0, 40.0), 0.0);
    }

    #[test]
    fn temp_gradient_level_above_hottest_returns_100() {
        // py:183-184  converted_temp >= temp_hottest: 100
        assert_eq!(temp_gradient_level(50.0, -30.0, 40.0), 100.0);
        assert_eq!(temp_gradient_level(40.0, -30.0, 40.0), 100.0);
    }

    #[test]
    fn temp_gradient_level_midpoint_returns_50() {
        // py:185-187  (temp - cold) * 100 / (hot - cold)
        let r = temp_gradient_level(5.0, -30.0, 40.0);
        // (5 - -30) * 100 / 70 = 35 * 100 / 70 = 50
        assert!((r - 50.0).abs() < 1e-9);
    }

    #[test]
    fn render_one_no_weather_returns_none() {
        let r = render_one(None, None, "C", None, -30.0, 40.0);
        assert!(r.is_none());
    }

    #[test]
    fn render_one_emits_two_segments() {
        // py:189-201  return [icon_segment, temp_segment]
        let r = render_one(Some((295.15, vec!["sunny"])), None, "C", None, -30.0, 40.0).unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn render_one_icon_segment_appends_trailing_space() {
        let r = render_one(Some((295.15, vec!["sunny"])), None, "C", None, -30.0, 40.0).unwrap();
        let contents = r[0]["contents"].as_str().unwrap();
        assert!(contents.ends_with(' '));
        assert_eq!(contents, "SUN ");
    }

    #[test]
    fn render_one_temp_segment_emits_gradient_level() {
        let r = render_one(Some((295.15, vec!["sunny"])), None, "C", None, -30.0, 40.0).unwrap();
        // 295.15 K → 22 °C → (22 - -30)/(40 - -30) * 100 = 52/70 * 100 ≈ 74.28
        let level = r[1]["gradient_level"].as_f64().unwrap();
        assert!((level - 74.285_714).abs() < 1e-3);
    }

    #[test]
    fn render_one_temp_segment_renders_default_format() {
        // py:177  default '{temp:.0f}' + unit symbol
        let r = render_one(Some((295.15, vec!["sunny"])), None, "C", None, -30.0, 40.0).unwrap();
        let temp_contents = r[1]["contents"].as_str().unwrap();
        // 295.15 - 273.15 = 22 → "22°C"
        assert_eq!(temp_contents, "22°C");
    }

    #[test]
    fn render_one_custom_temp_format() {
        let r = render_one(
            Some((295.15, vec!["sunny"])),
            None,
            "C",
            Some("temp: {temp:.0f}"),
            -30.0,
            40.0,
        )
        .unwrap();
        assert_eq!(r[1]["contents"], "temp: 22");
    }

    #[test]
    fn render_one_emits_weather_condition_groups() {
        // py:188  groups = ['weather_condition_' + n for n in icon_names] + ...
        let r = render_one(Some((295.15, vec!["sunny"])), None, "C", None, -30.0, 40.0).unwrap();
        let groups = r[0]["highlight_groups"].as_array().unwrap();
        assert_eq!(groups[0], "weather_condition_sunny");
        assert_eq!(groups[1], "weather_conditions");
        assert_eq!(groups[2], "weather");
    }

    #[test]
    fn render_one_temp_segment_groups_match_upstream() {
        // py:194  highlight_groups: ['weather_temp_gradient', 'weather_temp', 'weather']
        let r = render_one(Some((295.15, vec!["sunny"])), None, "C", None, -30.0, 40.0).unwrap();
        let groups = r[1]["highlight_groups"].as_array().unwrap();
        assert_eq!(groups[0], "weather_temp_gradient");
        assert_eq!(groups[1], "weather_temp");
        assert_eq!(groups[2], "weather");
    }

    #[test]
    fn render_one_uses_custom_icon_override() {
        let mut icons = Map::new();
        icons.insert("sunny".to_string(), Value::String("☀".into()));
        let r = render_one(
            Some((295.15, vec!["sunny"])),
            Some(&icons),
            "C",
            None,
            -30.0,
            40.0,
        )
        .unwrap();
        assert_eq!(r[0]["contents"], "☀ ");
    }

    #[test]
    fn render_one_emits_background_divider_on_both() {
        // py:191, 197  divider_highlight_group: 'background:divider'
        let r = render_one(Some((295.15, vec!["sunny"])), None, "C", None, -30.0, 40.0).unwrap();
        assert_eq!(r[0]["divider_highlight_group"], "background:divider");
        assert_eq!(r[1]["divider_highlight_group"], "background:divider");
    }
}
