use super::{
    Catalog, Device, INSTANCE_PROPERTY, Preferences, instance, owns_stream, target, unavailable,
};
use anyhow::{Context as _, Result, ensure};
use libpulse_binding::{
    callbacks::ListResult,
    context::{Context, FlagSet, State},
    error::Code,
    mainloop::standard::{IterateResult, Mainloop},
    operation::{Operation, State as OperationState},
};
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
    time::{Duration, Instant},
};

pub(super) struct Connection {
    context: Context,
    mainloop: Mainloop,
}

impl Connection {
    pub fn connect() -> Result<Self> {
        let mainloop = Mainloop::new().context(unavailable())?;
        let mut context =
            Context::new(&mainloop, "simpleVoiceover devices").context(unavailable())?;
        context
            .connect(None, FlagSet::NOAUTOSPAWN, None)
            .map_err(|_| anyhow::anyhow!(unavailable()))?;
        let mut connection = Self { context, mainloop };
        let deadline = Instant::now() + Duration::from_secs(2);
        while connection.context.get_state() != State::Ready {
            ensure!(
                !matches!(
                    connection.context.get_state(),
                    State::Failed | State::Terminated
                ),
                unavailable()
            );
            ensure!(Instant::now() < deadline, unavailable());
            connection.iterate()?;
        }
        Ok(connection)
    }

    fn iterate(&mut self) -> Result<()> {
        ensure!(
            !matches!(
                self.mainloop.iterate(false),
                IterateResult::Err(_) | IterateResult::Quit(_)
            ),
            unavailable()
        );
        std::thread::sleep(Duration::from_millis(1));
        Ok(())
    }

    fn wait<F: ?Sized>(&mut self, mut operation: Operation<F>) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(2);
        while operation.get_state() == OperationState::Running {
            if self.context.get_state() != State::Ready || Instant::now() >= deadline {
                operation.cancel();
                anyhow::bail!(unavailable());
            }
            if let Err(error) = self.iterate() {
                operation.cancel();
                return Err(error);
            }
        }
        ensure!(
            operation.get_state() == OperationState::Done
                && self.context.get_state() == State::Ready,
            unavailable()
        );
        Ok(())
    }

    pub fn synchronize(&mut self, preferences: &Preferences) -> Result<Catalog> {
        ensure!(self.context.get_state() == State::Ready, unavailable());
        let catalog = self.catalog()?;
        let clients = self.owned_clients()?;
        if let Some(output) = target(&catalog.outputs, &preferences.output) {
            let output_index = output.index;
            let streams = Rc::new(RefCell::new(Vec::new()));
            let collected = streams.clone();
            let failed = Rc::new(Cell::new(false));
            let callback_failed = failed.clone();
            let owned = clients.clone();
            let operation = self
                .context
                .introspect()
                .get_sink_input_info_list(move |result| {
                    if matches!(result, ListResult::Error) {
                        callback_failed.set(true);
                    }
                    if let ListResult::Item(stream) = result {
                        if owns_stream(
                            stream.proplist.get_str(INSTANCE_PROPERTY).as_deref(),
                            stream.client,
                            &owned,
                        ) && stream.sink != output_index
                        {
                            collected.borrow_mut().push(stream.index);
                        }
                    }
                });
            self.wait(operation)?;
            ensure!(!failed.get(), unavailable());
            for stream in streams.borrow().iter().copied() {
                let success = Rc::new(Cell::new(false));
                let callback_success = success.clone();
                let operation = self.context.introspect().move_sink_input_by_index(
                    stream,
                    output.index,
                    Some(Box::new(move |value| callback_success.set(value))),
                );
                self.wait(operation)?;
                ensure!(
                    success.get() || Code::try_from(self.context.errno()) == Ok(Code::NoEntity),
                    unavailable()
                );
            }
        }
        if let Some(input) = target(&catalog.inputs, &preferences.input) {
            let input_index = input.index;
            let streams = Rc::new(RefCell::new(Vec::new()));
            let collected = streams.clone();
            let failed = Rc::new(Cell::new(false));
            let callback_failed = failed.clone();
            let owned = clients;
            let operation = self
                .context
                .introspect()
                .get_source_output_info_list(move |result| {
                    if matches!(result, ListResult::Error) {
                        callback_failed.set(true);
                    }
                    if let ListResult::Item(stream) = result {
                        if owns_stream(
                            stream.proplist.get_str(INSTANCE_PROPERTY).as_deref(),
                            stream.client,
                            &owned,
                        ) && stream.source != input_index
                        {
                            collected.borrow_mut().push(stream.index);
                        }
                    }
                });
            self.wait(operation)?;
            ensure!(!failed.get(), unavailable());
            for stream in streams.borrow().iter().copied() {
                let success = Rc::new(Cell::new(false));
                let callback_success = success.clone();
                let operation = self.context.introspect().move_source_output_by_index(
                    stream,
                    input.index,
                    Some(Box::new(move |value| callback_success.set(value))),
                );
                self.wait(operation)?;
                ensure!(
                    success.get() || Code::try_from(self.context.errno()) == Ok(Code::NoEntity),
                    unavailable()
                );
            }
        }
        Ok(catalog)
    }

    fn catalog(&mut self) -> Result<Catalog> {
        let defaults = Rc::new(RefCell::new((String::new(), String::new())));
        let collected = defaults.clone();
        let operation = self.context.introspect().get_server_info(move |server| {
            *collected.borrow_mut() = (
                server
                    .default_source_name
                    .as_deref()
                    .unwrap_or_default()
                    .into(),
                server
                    .default_sink_name
                    .as_deref()
                    .unwrap_or_default()
                    .into(),
            );
        });
        self.wait(operation)?;
        let inputs = Rc::new(RefCell::new(Vec::new()));
        let collected = inputs.clone();
        let failed = Rc::new(Cell::new(false));
        let callback_failed = failed.clone();
        let operation = self
            .context
            .introspect()
            .get_source_info_list(move |result| {
                if matches!(result, ListResult::Error) {
                    callback_failed.set(true);
                }
                if let ListResult::Item(source) = result {
                    if let Some(name) = source.name.as_deref() {
                        collected.borrow_mut().push(Device {
                            id: name.into(),
                            label: source.description.as_deref().unwrap_or(name).into(),
                            index: source.index,
                            is_default: false,
                        });
                    }
                }
            });
        self.wait(operation)?;
        ensure!(!failed.get(), unavailable());
        let outputs = Rc::new(RefCell::new(Vec::new()));
        let collected = outputs.clone();
        let failed = Rc::new(Cell::new(false));
        let callback_failed = failed.clone();
        let operation = self.context.introspect().get_sink_info_list(move |result| {
            if matches!(result, ListResult::Error) {
                callback_failed.set(true);
            }
            if let ListResult::Item(sink) = result {
                if let Some(name) = sink.name.as_deref() {
                    collected.borrow_mut().push(Device {
                        id: name.into(),
                        label: sink.description.as_deref().unwrap_or(name).into(),
                        index: sink.index,
                        is_default: false,
                    });
                }
            }
        });
        self.wait(operation)?;
        ensure!(!failed.get(), unavailable());
        let mut catalog = Catalog {
            inputs: inputs.take(),
            outputs: outputs.take(),
            native_routing: true,
            available: true,
        };
        for (devices, name) in [
            (&mut catalog.inputs, &defaults.borrow().0),
            (&mut catalog.outputs, &defaults.borrow().1),
        ] {
            for device in devices.iter_mut() {
                device.is_default = device.id == *name;
            }
            devices
                .sort_by(|left, right| left.label.cmp(&right.label).then(left.id.cmp(&right.id)));
        }
        Ok(catalog)
    }

    fn owned_clients(&mut self) -> Result<HashSet<u32>> {
        let clients = Rc::new(RefCell::new(HashSet::new()));
        let collected = clients.clone();
        let failed = Rc::new(Cell::new(false));
        let callback_failed = failed.clone();
        let operation = self
            .context
            .introspect()
            .get_client_info_list(move |result| {
                if matches!(result, ListResult::Error) {
                    callback_failed.set(true);
                }
                if let ListResult::Item(client) = result {
                    if client.proplist.get_str(INSTANCE_PROPERTY).as_deref() == Some(instance()) {
                        collected.borrow_mut().insert(client.index);
                    }
                }
            });
        self.wait(operation)?;
        ensure!(!failed.get(), unavailable());
        Ok(clients.take())
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.context.disconnect();
    }
}
