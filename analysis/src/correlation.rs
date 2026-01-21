pub fn pearson(points: &[(f64, f64)]) -> f64 {
    let n = points.len() as f64;

    let mean_x = points.iter().map(|p| p.0).sum::<f64>() / n;
    let mean_y = points.iter().map(|p| p.1).sum::<f64>() / n;

    let mut cov = 0.0;
    let mut dx = 0.0;
    let mut dy = 0.0;

    for (x, y) in points {
        let a = x - mean_x;
        let b = y - mean_y;
        cov += a * b;
        dx += a * a;
        dy += b * b;
    }

    cov / (dx.sqrt() * dy.sqrt())
}

pub fn pearson_log(points: &[(f64, f64)]) -> f64 {
    let mut log_points = points.to_owned();
    for (x, y) in log_points.iter_mut() {
        *x = x.log10();
        *y = y.log10();
    }
    pearson(points)
}