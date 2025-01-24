use canister_time::timestamp_nanos;

pub fn get_current_day() -> u64 {
    timestamp_nanos() / 86400 / 1_000_000_000
}

pub fn fill_missing_days<T: Clone>(
    mut history: Vec<(u64, T)>,
    days: u64,
    default_data: T,
) -> Vec<(u64, T)> {
    history.sort_by_key(|&(day, _)| day);

    let mut filled_history = Vec::new();
    let current_day = get_current_day();

    let mut last_data = default_data.clone();
    let mut history_index = 0;

    for day_offset in (0..=days).rev() {
        let day = current_day - day_offset;

        while history_index < history.len() && history[history_index].0 <= day {
            last_data = history[history_index].1.clone();
            history_index += 1;
        }

        filled_history.push((day, last_data.clone()));
    }

    filled_history
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fill_missing_days_continuous() {
        let history = vec![
            (get_current_day() - 4, 100),
            (get_current_day() - 3, 200),
            (get_current_day() - 2, 300),
            (get_current_day() - 1, 400),
            (get_current_day(), 500),
        ];
        let filled = fill_missing_days(history, 5, 0);
        let expected = vec![
            (get_current_day() - 5, 0),
            (get_current_day() - 4, 100),
            (get_current_day() - 3, 200),
            (get_current_day() - 2, 300),
            (get_current_day() - 1, 400),
            (get_current_day(), 500),
        ];
        assert_eq!(filled, expected);
    }

    #[test]
    fn test_fill_missing_days_with_gaps() {
        let history = vec![
            (get_current_day() - 4, 100),
            (get_current_day() - 2, 200),
            (get_current_day(), 300),
        ];
        let filled = fill_missing_days(history, 5, 0);
        let expected = vec![
            (get_current_day() - 5, 0),
            (get_current_day() - 4, 100),
            (get_current_day() - 3, 100),
            (get_current_day() - 2, 200),
            (get_current_day() - 1, 200),
            (get_current_day(), 300),
        ];
        assert_eq!(filled, expected);
    }

    #[test]
    fn test_fill_missing_days_empty_history() {
        let history: Vec<(u64, i32)> = Vec::new();
        let filled = fill_missing_days(history, 5, 0);
        let expected = vec![
            (get_current_day() - 5, 0),
            (get_current_day() - 4, 0),
            (get_current_day() - 3, 0),
            (get_current_day() - 2, 0),
            (get_current_day() - 1, 0),
            (get_current_day(), 0),
        ];
        assert_eq!(filled, expected);
    }

    #[test]
    fn test_fill_missing_days_partial_history() {
        let history = vec![(get_current_day() - 2, 200), (get_current_day() - 1, 300)];
        let filled = fill_missing_days(history, 5, 0);
        let expected = vec![
            (get_current_day() - 5, 0),
            (get_current_day() - 4, 0),
            (get_current_day() - 3, 0),
            (get_current_day() - 2, 200),
            (get_current_day() - 1, 300),
            (get_current_day(), 300),
        ];
        assert_eq!(filled, expected);
    }

    #[test]
    fn test_fill_missing_days_with_old_history() {
        let history = vec![(get_current_day() - 49, 300)];
        let filled = fill_missing_days(history, 5, 0);
        let expected = vec![
            (get_current_day() - 5, 300),
            (get_current_day() - 4, 300),
            (get_current_day() - 3, 300),
            (get_current_day() - 2, 300),
            (get_current_day() - 1, 300),
            (get_current_day(), 300),
        ];
        assert_eq!(filled, expected);
    }
}
