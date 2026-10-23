//! Datasets (K4).
//!
//! Every instrument and analysis output is a [`Dataset`]: named, unit-carrying
//! columns plus the dimensions they are indexed by. A time series is the
//! special case with one `Time` axis.

use std::fmt::Write as _;

use cetus_compile::{CompiledPlan, StateLayout};
use cetus_units::Unit;

/// One dimension of a dataset.
///
/// Axes are few (one to four per dataset), so the size difference between
/// variants does not justify boxing; `Unit` carries a `String` plus a `Dim`.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum DimAxis {
    /// Time, in ticks of the plan's time base.
    Time {
        /// Tick values.
        ticks: Vec<u64>,
        /// Seconds per display unit (for converting ticks to display time).
        unit_factor: f64,
        /// Seconds per tick.
        quantum: f64,
    },
    /// A swept parameter.
    Parameter {
        /// Parameter name.
        id: String,
        /// Values in SI units.
        values: Vec<f64>,
        /// Display unit.
        unit: Unit,
        /// Multiplicative factor to SI.
        factor: f64,
    },
    /// One ensemble member per seed.
    Ensemble {
        /// Seeds.
        seeds: Vec<u64>,
    },
}

impl DimAxis {
    /// Number of points along this axis.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Time { ticks, .. } => ticks.len(),
            Self::Parameter { values, .. } => values.len(),
            Self::Ensemble { seeds } => seeds.len(),
        }
    }

    /// Whether the axis is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The axis name used in CSV headers.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Time { .. } => "time",
            Self::Parameter { id, .. } => id,
            Self::Ensemble { .. } => "seed",
        }
    }

    /// The value at `index`, already converted to the display unit.
    #[must_use]
    pub fn value_at(&self, index: usize) -> f64 {
        match self {
            Self::Time {
                ticks,
                unit_factor,
                quantum,
                ..
            } => ticks
                .get(index)
                .map_or(0.0, |tick| *tick as f64 * quantum / unit_factor),
            Self::Parameter { values, factor, .. } => {
                values.get(index).map_or(0.0, |value| value / factor)
            }
            Self::Ensemble { seeds } => seeds.get(index).map_or(0.0, |seed| *seed as f64),
        }
    }
}

/// A named, unit-carrying column of values.
#[derive(Debug, Clone)]
pub struct Column {
    /// Column name.
    pub name: String,
    /// Display unit.
    pub unit: Unit,
    /// Values in SI units, row-major over the dimensions.
    pub values: Vec<f64>,
}

impl Column {
    /// Value at a flat row index, converted to the display unit.
    #[must_use]
    pub fn display_value(&self, index: usize) -> f64 {
        self.values
            .get(index)
            .map_or(f64::NAN, |value| value / self.unit.factor())
    }
}

/// A multi-dimensional dataset.
#[derive(Debug, Clone)]
pub struct Dataset {
    /// Dimensions, outermost first.
    pub dims: Vec<DimAxis>,
    /// Columns sharing the same row order.
    pub columns: Vec<Column>,
}

impl Dataset {
    /// Builds a dataset from probe buffers (one `Time` dimension).
    #[must_use]
    pub fn from_probes(plan: &CompiledPlan, probes: &[crate::Probe]) -> Self {
        let time = DimAxis::Time {
            ticks: probes
                .first()
                .map_or_else(Vec::new, |probe| probe.ticks.clone()),
            unit_factor: plan.time.unit_factor,
            quantum: plan.time.quantum,
        };
        let mut columns = Vec::with_capacity(probes.len());
        for probe in probes {
            let unit = plan
                .observables
                .iter()
                .find(|observable| observable.name == probe.name)
                .map_or_else(
                    || Unit::linear("1", cetus_units::Dim::dimensionless(), 1.0),
                    |observable| observable.unit.clone(),
                );
            columns.push(Column {
                name: probe.name.clone(),
                unit,
                values: probe.values.clone(),
            });
        }
        Self {
            dims: vec![time],
            columns,
        }
    }

    /// Number of rows (product of dimension lengths).
    #[must_use]
    pub fn rows(&self) -> usize {
        self.dims.iter().map(DimAxis::len).product()
    }

    /// Row-major CSV, one header line, values in display units.
    #[must_use]
    pub fn to_csv(&self) -> String {
        let mut out = String::new();
        for (index, dim) in self.dims.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            let _ = write!(out, "{}", dim.name());
        }
        for column in &self.columns {
            let _ = write!(out, ",{}", column.name);
        }
        out.push('\n');

        let total = self.rows();
        for row in 0..total {
            let mut first = true;
            let mut stride = total;
            for dim in &self.dims {
                let length = dim.len().max(1);
                stride /= length;
                let index = (row / stride) % length;
                if !first {
                    out.push(',');
                }
                first = false;
                let _ = write!(out, "{}", dim.value_at(index));
            }
            for column in &self.columns {
                let _ = write!(out, ",{}", column.display_value(row));
            }
            out.push('\n');
        }
        out
    }
}

/// Reserved for the ensemble analysis (M3).
#[must_use]
pub fn layout_size(layout: &StateLayout) -> usize {
    layout.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_has_a_header_and_one_row_per_point() {
        let unit = Unit::linear("1", cetus_units::Dim::dimensionless(), 1.0);
        let dataset = Dataset {
            dims: vec![DimAxis::Time {
                ticks: vec![0, 1, 2],
                unit_factor: 3600.0,
                quantum: 1800.0,
            }],
            columns: vec![Column {
                name: "x".to_owned(),
                unit,
                values: vec![1.0, 2.0, 3.0],
            }],
        };
        let csv = dataset.to_csv();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "time,x");
        assert_eq!(lines.len(), 4);
        assert!(lines[1].starts_with("0,"));
        assert!(lines[3].starts_with("1,"));
    }
}
