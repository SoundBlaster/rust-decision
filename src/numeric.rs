pub(crate) fn unit(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}
// Canonical order and Neumaier compensation make sums independent of input order.
pub(crate) fn sum(mut values: Vec<f64>) -> f64 {
    values.sort_unstable_by(f64::total_cmp);
    let (mut sum, mut correction) = (0.0_f64, 0.0_f64);
    for value in values {
        let next = sum + value;
        correction += if sum.abs() >= value.abs() {
            (sum - next) + value
        } else {
            (value - next) + sum
        };
        sum = next;
    }
    sum + correction
}
