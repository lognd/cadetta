//! ISO 286-1 / 286-2 tables: IT grades and shaft fundamental deviations.
//!
//! Size ranges are (lo, hi] in millimetres. IT grades use the 13 main
//! ranges; the `c` deviation uses the finer 25-range split.

/// Upper bounds (mm) of the 13 main ranges used by IT grades.
const COARSE_MM: [i64; 13] = [3, 6, 10, 18, 30, 50, 80, 120, 180, 250, 315, 400, 500];

/// Upper bounds (mm) of the 25 fine ranges.
const FINE_MM: [i64; 25] = [
    3, 6, 10, 14, 18, 24, 30, 40, 50, 65, 80, 100, 120, 140, 160, 180, 200, 225, 250, 280, 315,
    355, 400, 450, 500,
];

/// A located size range.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Range {
    /// Index into the 13 main ranges.
    pub coarse: usize,
    /// Index into the 25 fine ranges.
    pub fine: usize,
}

/// Finds the range containing `size_nm`, or `None` outside (0, 500] mm.
pub(crate) fn fine_range(size_nm: i64) -> Option<Range> {
    if size_nm <= 0 {
        return None;
    }
    let fine = FINE_MM.iter().position(|&hi| size_nm <= hi * 1_000_000)?;
    let coarse = COARSE_MM.iter().position(|&hi| FINE_MM[fine] <= hi)?;
    Some(Range { coarse, fine })
}

/// IT1..IT18 in tenths of a micrometre, one row per grade, by main range.
#[rustfmt::skip]
const IT_TENTHS_UM: [[i64; 13]; 18] = [
    [8, 10, 10, 12, 15, 15, 20, 25, 35, 45, 60, 70, 80],
    [12, 15, 15, 20, 25, 25, 30, 40, 50, 70, 80, 90, 100],
    [20, 25, 25, 30, 40, 40, 50, 60, 80, 100, 120, 130, 150],
    [30, 40, 40, 50, 60, 70, 80, 100, 120, 140, 160, 180, 200],
    [40, 50, 60, 80, 90, 110, 130, 150, 180, 200, 230, 250, 270],
    [60, 80, 90, 110, 130, 160, 190, 220, 250, 290, 320, 360, 400],
    [100, 120, 150, 180, 210, 250, 300, 350, 400, 460, 520, 570, 630],
    [140, 180, 220, 270, 330, 390, 460, 540, 630, 720, 810, 890, 970],
    [250, 300, 360, 430, 520, 620, 740, 870, 1000, 1150, 1300, 1400, 1550],
    [400, 480, 580, 700, 840, 1000, 1200, 1400, 1600, 1850, 2100, 2300, 2500],
    [600, 750, 900, 1100, 1300, 1600, 1900, 2200, 2500, 2900, 3200, 3600, 4000],
    [1000, 1200, 1500, 1800, 2100, 2500, 3000, 3500, 4000, 4600, 5200, 5700, 6300],
    [1400, 1800, 2200, 2700, 3300, 3900, 4600, 5400, 6300, 7200, 8100, 8900, 9700],
    [2500, 3000, 3600, 4300, 5200, 6200, 7400, 8700, 10000, 11500, 13000, 14000, 15500],
    [4000, 4800, 5800, 7000, 8400, 10000, 12000, 14000, 16000, 18500, 21000, 23000, 25000],
    [6000, 7500, 9000, 11000, 13000, 16000, 19000, 22000, 25000, 29000, 32000, 36000, 40000],
    [10000, 12000, 15000, 18000, 21000, 25000, 30000, 35000, 40000, 46000, 52000, 57000, 63000],
    [14000, 18000, 22000, 27000, 33000, 39000, 46000, 54000, 63000, 72000, 81000, 89000, 97000],
];

/// IT tolerance width in nanometres for a 1..=18 `grade`.
pub(crate) fn it_nm(coarse: usize, grade: u8) -> i64 {
    IT_TENTHS_UM[usize::from(grade) - 1][coarse] * 100
}

/// Shaft `c` upper deviation es (um) by fine range.
const C_ES: [i64; 25] = [
    -60, -70, -80, -95, -95, -110, -110, -120, -130, -140, -150, -170, -180, -200, -210, -230,
    -240, -260, -280, -300, -330, -360, -400, -440, -480,
];
/// Shaft `d` es (um) by main range.
const D_ES: [i64; 13] = [
    -20, -30, -40, -50, -65, -80, -100, -120, -145, -170, -190, -210, -230,
];
/// Shaft `e` es (um) by main range.
const E_ES: [i64; 13] = [
    -14, -20, -25, -32, -40, -50, -60, -72, -85, -100, -110, -125, -135,
];
/// Shaft `f` es (um) by main range.
const F_ES: [i64; 13] = [
    -6, -10, -13, -16, -20, -25, -30, -36, -43, -50, -56, -62, -68,
];
/// Shaft `g` es (um) by main range.
const G_ES: [i64; 13] = [-2, -4, -5, -6, -7, -9, -10, -12, -14, -15, -17, -18, -20];

/// Shaft `k` ei (um) for grades 4..=7, by main range.
const K_EI: [i64; 13] = [0, 1, 1, 1, 2, 2, 2, 3, 3, 4, 4, 4, 5];
/// Shaft `m` ei (um) by main range.
const M_EI: [i64; 13] = [2, 4, 6, 7, 8, 9, 11, 13, 15, 17, 20, 21, 23];
/// Shaft `n` ei (um) by main range.
const N_EI: [i64; 13] = [4, 8, 10, 12, 15, 17, 20, 23, 27, 31, 34, 37, 40];
/// Shaft `p` ei (um) by main range.
const P_EI: [i64; 13] = [6, 12, 15, 18, 22, 26, 32, 37, 43, 50, 56, 62, 68];
/// Shaft `j5`/`j6` ei (um) by main range.
const J56_EI: [i64; 13] = [-2, -2, -2, -3, -4, -5, -7, -9, -11, -13, -16, -18, -20];
/// Shaft `j7` ei (um) by main range.
const J7_EI: [i64; 13] = [-4, -4, -5, -6, -8, -10, -12, -15, -18, -21, -26, -28, -32];

/// Upper deviation es (um) of shaft letters `c d e f g h`; `None` otherwise.
pub(crate) fn shaft_upper_um(letter: &str, r: Range) -> Option<i64> {
    Some(match letter {
        "c" => C_ES[r.fine],
        "d" => D_ES[r.coarse],
        "e" => E_ES[r.coarse],
        "f" => F_ES[r.coarse],
        "g" => G_ES[r.coarse],
        "h" => 0,
        _ => return None,
    })
}

/// Lower deviation ei (um) of shaft letters `j k m n p`; `None` if untabulated.
pub(crate) fn shaft_lower_um(letter: &str, grade: u8, r: Range) -> Option<i64> {
    Some(match letter {
        "j" => match grade {
            5 | 6 => J56_EI[r.coarse],
            7 => J7_EI[r.coarse],
            _ => return None,
        },
        "k" if (4..=7).contains(&grade) => K_EI[r.coarse],
        "k" => 0,
        "m" => M_EI[r.coarse],
        "n" => N_EI[r.coarse],
        "p" => P_EI[r.coarse],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // frob:ticket 01M4T1E2SJJS6GBRWKFDRWZP5A
    /// IT1..IT4 match ISO 286-1 table 1 across all main ranges (regression for over-30 mm rows).
    #[test]
    fn it1_to_it4_match_iso286_table1() {
        let expect: [[i64; 13]; 4] = [
            [8, 10, 10, 12, 15, 15, 20, 25, 35, 45, 60, 70, 80],
            [12, 15, 15, 20, 25, 25, 30, 40, 50, 70, 80, 90, 100],
            [20, 25, 25, 30, 40, 40, 50, 60, 80, 100, 120, 130, 150],
            [30, 40, 40, 50, 60, 70, 80, 100, 120, 140, 160, 180, 200],
        ];
        for (g, row) in expect.iter().enumerate() {
            for (c, &tenths) in row.iter().enumerate() {
                assert_eq!(it_nm(c, g as u8 + 1), tenths * 100, "IT{} range {c}", g + 1);
            }
        }
        // 30-50 mm is coarse index 5.
        assert_eq!(it_nm(5, 3), 4_000);
        assert_eq!(it_nm(5, 4), 7_000);
    }
}
