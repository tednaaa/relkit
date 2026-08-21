use std::time::{SystemTime, UNIX_EPOCH};

const SECONDS_PER_DAY: u64 = 86_400;

pub fn today_utc() -> String {
	let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs());
	let (year, month, day) = civil_from_days(i64::try_from(seconds / SECONDS_PER_DAY).unwrap_or(0));

	format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days: i64) -> (i64, u64, u64) {
	let shifted = days + 719_468;
	let era = if shifted >= 0 { shifted } else { shifted - 146_096 } / 146_097;
	let day_of_era = u64::try_from(shifted - era * 146_097).unwrap_or(0);
	let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
	let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
	let month_index = (5 * day_of_year + 2) / 153;

	let day = day_of_year - (153 * month_index + 2) / 5 + 1;
	let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
	let year = i64::try_from(year_of_era).unwrap_or(0) + era * 400 + i64::from(month <= 2);

	(year, month, day)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn converts_known_days() {
		assert_eq!(civil_from_days(0), (1970, 1, 1));
		assert_eq!(civil_from_days(19_723), (2024, 1, 1));
		assert_eq!(civil_from_days(19_782), (2024, 2, 29));
		assert_eq!(civil_from_days(20_686), (2026, 8, 21));
	}

	#[test]
	fn formats_as_iso_date() {
		let today = today_utc();

		assert_eq!(today.len(), 10);
		assert!(today.chars().enumerate().all(|(index, char)| if matches!(index, 4 | 7) {
			char == '-'
		} else {
			char.is_ascii_digit()
		}));
	}
}
