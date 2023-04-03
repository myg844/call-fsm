#[cfg(test)]
mod tests {
    use std::time::Duration;
    use call_fsm::{*};

    use chrono::prelude::*;
    use chrono::format::{DelayedFormat, StrftimeItems};

    #[derive(Clone, Copy)]
    struct Status {
        pub st_u32: u32,
        pub st_i32: i32,
        pub st_bool: bool,
    }

    fn init_sm() -> StateMachine<Status> {
        let status = Status { st_u32: 5, st_i32: -6, st_bool: true };

        declare_data_type!(Status);
        declare_state_machine!(sm, status, 3);

        new_state!(sm, state1,
            &generic_state_init,
            &generic_state_exec
        );

        // let name: State = State::new(stringify!(name), &state1_init, &(|s: &State| { println!("exec {}", s.name) }));
        // sm.add_state(name);
        // let name = sm.states().last().unwrap();

        new_state!(sm, state2,
            &generic_state_init,
            &generic_state_exec
        );

        new_state!(sm, state3,
            &generic_state_init,
            &generic_state_exec
        );

        new_transition!(sm,
            state1,
            state2,
            &generic_trans_check,
            &generic_trans_done
        );

        new_transition!(sm,
            state2,
            state3,
            &generic_trans_check,
            &generic_trans_done
        );

        new_transition!(sm,
            state3,
            state1,
            &generic_trans_check,
            &generic_trans_done
        );

        sm.set_error_callbacks(&error_init, &error_exec);

        sm
    }

    fn now() -> DelayedFormat<StrftimeItems<'static>> {
        Local::now().format("%Y-%m-%d %H:%M:%S.%3f")
    }

    fn generic_state_init(s: &State<Status>, data: &mut Status) -> Result<(), FsmError> {
        println!("{} ::: init {} ::: {}", now(), s.name, data.st_bool);
        data.st_bool = !data.st_bool;
        Ok(())
    }

    fn generic_state_exec(s: &State<Status>, data: &mut Status) -> Result<(), FsmError> {
        std::thread::sleep(Duration::from_secs(1));
        println!("{} ::: exec {} ::: {}", now(), s.name, data.st_i32);
        data.st_i32 += 1;
        Ok(())
    }

    fn generic_trans_check(t: &Transition<Status>, data: &Status) -> bool {
        println!("{} ::: check {} ::: {}", now(), t.name, data.st_bool);
        true
    }

    fn generic_trans_done(t: &Transition<Status>, data: &mut Status) -> Result<(), FsmError> {
        println!("{} ::: done {} ::: {}", now(), t.name, data.st_u32);
        data.st_u32 += 1;
        // Err(FsmError::StateIsEmpty)
        Ok(())
    }

    fn error_init(error: FsmError, data: &mut Status) -> Option<Destination> {
        println!("{} ::: error init {} ::: {}", now(), error, data.st_u32);
        None
    }

    fn error_exec(error: FsmError, data: &mut Status) -> Option<Destination> {
        println!("{} ::: error exec {} ::: {}", now(), error, data.st_u32);
        Some(Destination::Name(String::from("state2")))
    }

    #[test]
    fn simple_test() {
        let mut sm = init_sm();
        sm.set_active_state(0).unwrap();

        loop {
            sm.run();
        }
    }
}