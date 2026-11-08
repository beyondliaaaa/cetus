//! Initial-value overrides for a new simulation.

use cetus_compile::CompiledPlan;
use cetus_units::Unit;
use indexmap::IndexMap;

use crate::SimError;

/// A value in a declared unit.
#[derive(Debug, Clone)]
pub struct Quantity {
    /// Value in [`Quantity::unit`].
    pub value: f64,
    /// Unit the value is expressed in.
    pub unit: Unit,
}

/// A stock initial-value override.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum InitialValue {
    /// A value in its declared unit.
    Value(Quantity),
    /// An expression over model parameters, evaluated under the selected parameters.
    Expression(String),
}

/// Parameters and stock initial values applied when a [`crate::Sim`] is created.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct SimInit {
    /// Parameter overrides, in their declared units.
    pub parameters: IndexMap<String, Quantity>,
    /// Stock initial-value overrides.
    pub stock_initial: IndexMap<String, InitialValue>,
}

impl SimInit {
    /// An empty override set: use the plan's defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a parameter override expressed in SI units.
    ///
    /// # Errors
    /// Returns an error for an unknown parameter or a value outside its range.
    pub fn set_parameter_si(
        &mut self,
        plan: &CompiledPlan,
        name: &str,
        si_value: f64,
    ) -> Result<(), SimError> {
        let Some(parameter) = plan
            .parameters
            .iter()
            .find(|parameter| parameter.name == name)
        else {
            return Err(SimError::UnknownName(name.to_owned()));
        };
        if let Some((min, max)) = parameter.range
            && !(si_value >= min && si_value <= max)
        {
            return Err(SimError::Subsystem(format!(
                "parameter `{name}` = {si_value} is outside [{min}, {max}]"
            )));
        }
        let Some(slot) = plan.layout.slots.get(parameter.slot) else {
            return Err(SimError::UnknownName(name.to_owned()));
        };
        self.parameters.insert(
            name.to_owned(),
            Quantity {
                value: slot.unit.from_si(si_value),
                unit: slot.unit.clone(),
            },
        );
        Ok(())
    }

    /// Adds a stock initial-value override.
    pub fn set_stock_initial(&mut self, name: impl Into<String>, value: InitialValue) {
        self.stock_initial.insert(name.into(), value);
    }
}
