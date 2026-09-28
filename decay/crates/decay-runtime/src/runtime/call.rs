//! Entering a function: instantiating, dispatching, and the depth limit.

use std::collections::HashMap;

use decay_ir::{IrContainer, IrFunction, IrProgram};

use crate::error::RuntimeError;
use crate::host::Host;
use crate::instance::{ScriptInstance, Slot};
use crate::value::Value;

use super::{Frame, Runtime};

impl<'a, H: Host> Runtime<'a, H> {
    pub fn instantiate(&mut self, container_name: &str) -> Result<ScriptInstance, RuntimeError> {
        self.begin_budget();
        let container = self.find_container(container_name)?;
        let fields = self.initialize_fields(container)?;
        Ok(ScriptInstance {
            container_name: container_name.to_owned(),
            fields,
        })
    }

    pub fn call(
        &mut self,
        container_name: &str,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        let mut instance = self.instantiate(container_name)?;
        self.call_instance(&mut instance, function_name, args)
    }

    pub fn call_instance(
        &mut self,
        instance: &mut ScriptInstance,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        self.begin_budget();
        let container = self.find_container(&instance.container_name)?;
        self.call_in_container(container, &mut instance.fields, function_name, args)
    }

    /// The container by name, borrowed from the program rather than from the
    /// runtime, so a call can hold it while the runtime runs it.
    ///
    /// Callers used to clone it for that reason, which copied every function
    /// and instruction of the script on every call and freed them after: about
    /// half of what running Orbital's scripts cost.
    pub(super) fn find_container(&self, name: &str) -> Result<&'a IrContainer, RuntimeError> {
        let program: &'a IrProgram = self.program;
        program
            .containers
            .iter()
            .find(|container| container.name == name)
            .ok_or_else(|| RuntimeError::ContainerNotFound(name.to_owned()))
    }

    pub(super) fn initialize_fields(
        &mut self,
        container: &IrContainer,
    ) -> Result<HashMap<String, Slot>, RuntimeError> {
        let mut fields = HashMap::new();
        for field in &container.fields {
            let value = if let Some(initializer) = &field.initializer {
                let mut frame = Frame::new(HashMap::new());
                self.execute_instructions(container, &mut fields, &mut frame, initializer)?
            } else {
                // A vector field nobody initialised starts at zero, as a
                // position or a velocity does; anything else starts absent.
                match field.type_name.as_deref() {
                    Some("Vec2") => Value::Vec2([0.0; 2]),
                    Some("Vec3") => Value::Vec3([0.0; 3]),
                    _ => Value::Null,
                }
            };
            fields.insert(
                field.name.clone(),
                Slot {
                    value,
                    mutable: field.mutable,
                },
            );
        }
        Ok(fields)
    }

    pub(super) fn call_in_container(
        &mut self,
        container: &IrContainer,
        fields: &mut HashMap<String, Slot>,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        let function = container
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .ok_or_else(|| RuntimeError::FunctionNotFound(function_name.to_owned()))?;
        if function.params.len() != args.len() {
            return Err(RuntimeError::Arity {
                function: function_name.to_owned(),
                expected: function.params.len(),
                found: args.len(),
            });
        }
        let locals = function
            .params
            .iter()
            .cloned()
            .zip(args)
            .map(|(name, value)| {
                (
                    name,
                    Slot {
                        value,
                        mutable: false,
                    },
                )
            })
            .collect();
        let mut frame = Frame::new(locals);

        // Counted here rather than around `execute_instructions`, because a
        // field initializer and a function body both run instructions and only
        // one of them is a call.
        if self.depth >= self.call_depth_limit {
            return Err(RuntimeError::CallDepthExceeded {
                function: function_name.to_owned(),
                limit: self.call_depth_limit,
            });
        }
        self.depth += 1;
        let result = self.execute_function(container, fields, function, &mut frame);
        self.depth -= 1;
        result
    }

    pub(super) fn execute_function(
        &mut self,
        container: &IrContainer,
        fields: &mut HashMap<String, Slot>,
        function: &IrFunction,
        frame: &mut Frame,
    ) -> Result<Value, RuntimeError> {
        self.execute_instructions(container, fields, frame, &function.instructions)
    }
}
