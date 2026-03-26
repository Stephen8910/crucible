/// Assert that a contract event was emitted.
///
/// This macro supports several forms for flexible event assertion:
///
/// - `assert_emitted!(env, topics: (...), data: value)`
/// - `assert_emitted!(env, contract: addr, topics: (...), data: value)`
/// - `assert_emitted!(env, topics: (...), count: n)`
/// - `assert_emitted!(env, topics: (...), at_index: n, data: value)`
///
/// Matching is a partial match on topics — all topics in the filter must be
/// present at the start of the event's topics.
#[macro_export]
macro_rules! assert_emitted {
    // Form 4: with at_index and data
    ($env:expr, topics: $topics:expr, at_index: $index:expr, data: $data:expr $(,)?) => {
        {
            let env = &$env;
            let topics = $topics;
            let matching = env.events_matching(topics.clone());
            let actual_count = matching.len();
            let index: u32 = $index;
            if index >= actual_count {
                panic!("Expected event at index {} for topics {:?}, but only found {}.\nActual emitted events: {:?}", index, topics, actual_count, env.events_all());
            }
            let event = matching.get(index).unwrap();
                        let expected_data: $crate::soroban_sdk::Val = $crate::soroban_sdk::IntoVal::into_val(&$data, env.inner());
                                                if format!("{:?}", event.2) != format!("{:?}", expected_data) {
                panic!("Event at index {} for topics {:?} had wrong data.\nExpected: {:?}\nActual: {:?}\nActual emitted events: {:?}", index, topics, $data, event.2, env.events_all());
            }
        }
    };
    // Form 3: with count
    ($env:expr, topics: $topics:expr, count: $count:expr $(,)?) => {
        {
            let env = &$env;
            let topics = $topics;
            let matching = env.events_matching(topics.clone());
            let expected_count: u32 = $count;
            if matching.len() != expected_count {
                panic!("Expected {} events matching topics {:?}, but found {}.\nActual emitted events: {:?}", expected_count, topics, matching.len(), env.events_all());
            }
        }
    };
    // Form 2: with contract, topics, and data
    ($env:expr, contract: $contract:expr, topics: $topics:expr, data: $data:expr $(,)?) => {
        {
            let env = &$env;
            let topics = $topics;
            let matching = env.events_matching(topics.clone());
            let contract_addr: $crate::soroban_sdk::Address = $contract.clone();
            let mut found = false;
            let mut found_count = 0;
                        let expected_data: $crate::soroban_sdk::Val = $crate::soroban_sdk::IntoVal::into_val(&$data, env.inner());
            for event in matching.iter() {
                if event.0 == contract_addr {
                    found_count += 1;
                                                                                if format!("{:?}", event.2) == format!("{:?}", expected_data) {
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                if found_count == 0 {
                    panic!("No events from contract {:?} matched topics {:?}.\nActual emitted events: {:?}", $contract, topics, env.events_all());
                } else {
                    panic!("None of the {} events from contract {:?} matching topics {:?} had correct data.\nExpected: {:?}\nActual emitted events: {:?}", found_count, $contract, topics, $data, env.events_all());
                }
            }
        }
    };
    // Form 1: topics and data
    ($env:expr, topics: $topics:expr, data: $data:expr $(,)?) => {
        {
            let env = &$env;
            let topics = $topics;
            let matching = env.events_matching(topics.clone());
            if matching.len() == 0 {
                panic!("Expected event matching topics {:?}, but found none.\nActual emitted events: {:?}", topics, env.events_all());
            }
                        let expected_data: $crate::soroban_sdk::Val = $crate::soroban_sdk::IntoVal::into_val(&$data, env.inner());
            let mut found = false;
                                                for event in matching.iter() {
                if format!("{:?}", event.2) == format!("{:?}", expected_data) {
                    found = true;
                    break;
                }
            }
            if !found {
                panic!("No event matching topics {:?} had data {:?}.\nActual emitted events: {:?}", topics, $data, env.events_all());
            }
        }
    };
}

/// Assert that a contract event was not emitted.
#[macro_export]
macro_rules! assert_not_emitted {
    // Basic topics version
    ($env:expr, topics: $topics:expr $(,)?) => {
        {
            let env = &$env;
            let topics = $topics;
            let matching = env.events_matching(topics.clone());
            if matching.len() > 0 {
                panic!("Expected no events matching topics {:?}, but found {}.\nActual emitted events: {:?}", topics, matching.len(), env.events_all());
            }
        }
    };
    // Contract filtered version
    ($env:expr, contract: $contract:expr, topics: $topics:expr $(,)?) => {
        {
            let env = &$env;
            let topics = $topics;
            let matching = env.events_matching(topics.clone());
            let contract_addr: $crate::soroban_sdk::Address = $contract.clone();
            let mut found_count = 0;
            for event in matching.iter() {
                if event.0 == contract_addr {
                    found_count += 1;
                }
            }
                        if found_count > 0 {
                panic!("Expected no events from contract {:?} matching topics {:?}, but found {}.\nActual emitted events: {:?}", $contract, topics, found_count, env.events_all());
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use soroban_sdk::{
        contract, contractimpl, symbol_short, testutils::Address as _, Address, Env,
    };

    #[contract]
    #[derive(Default, Debug)]
    pub struct TestContract;

    #[contractimpl]
    impl TestContract {
        pub fn fire(env: Env, value: u32) {
            env.events().publish((symbol_short!("test"),), value);
        }
        pub fn fire_multi(env: Env, value: u32) {
            env.events()
                .publish((symbol_short!("v1"), symbol_short!("data")), value);
            env.events()
                .publish((symbol_short!("v1"), symbol_short!("data")), value + 1);
        }
    }

    #[test]
    fn test_macro_emitted_success() {
        let env = MockEnv::builder().with_contract::<TestContract>().build();
        let contract_id = env.contract_id::<TestContract>();
        let client = TestContractClient::new(env.inner(), &contract_id);

        client.fire(&42);

        // Different forms
        assert_emitted!(env, topics: (symbol_short!("test"),), data: 42_u32);
        assert_emitted!(env, contract: &contract_id, topics: (symbol_short!("test"),), data: 42_u32);
        assert_emitted!(env, topics: (symbol_short!("test"),), count: 1);
        assert_emitted!(env, topics: (symbol_short!("test"),), at_index: 0, data: 42_u32);
    }

    #[test]
    fn test_macro_multi_success() {
        let env = MockEnv::builder().with_contract::<TestContract>().build();
        let contract_id = env.contract_id::<TestContract>();
        let client = TestContractClient::new(env.inner(), &contract_id);

        client.fire_multi(&10);

        assert_emitted!(env, topics: (symbol_short!("v1"),), count: 2);
        assert_emitted!(env, topics: (symbol_short!("v1"), symbol_short!("data")), at_index: 0, data: 10_u32);
        assert_emitted!(env, topics: (symbol_short!("v1"), symbol_short!("data")), at_index: 1, data: 11_u32);
    }

    #[test]
    fn test_macro_not_emitted_success() {
        let env = MockEnv::builder().with_contract::<TestContract>().build();
        let contract_id = env.contract_id::<TestContract>();
        let client = TestContractClient::new(env.inner(), &contract_id);

        client.fire(&42);

        assert_not_emitted!(env, topics: (symbol_short!("missing"),));
        assert_not_emitted!(env, contract: &Address::generate(env.inner()), topics: (symbol_short!("test"),));
    }

    #[test]
    #[should_panic(expected = "Expected 2 events matching topics (Symbol(test),)")]
    fn test_macro_panic_wrong_count() {
        let env = MockEnv::builder().with_contract::<TestContract>().build();
        let contract_id = env.contract_id::<TestContract>();
        let client = TestContractClient::new(env.inner(), &contract_id);

        client.fire(&42);

        assert_emitted!(env, topics: (symbol_short!("test"),), count: 2);
    }

    #[test]
    #[should_panic(expected = "No event matching topics (Symbol(test),) had data 43")]
    fn test_macro_panic_wrong_data() {
        let env = MockEnv::builder().with_contract::<TestContract>().build();
        let contract_id = env.contract_id::<TestContract>();
        let client = TestContractClient::new(env.inner(), &contract_id);

        client.fire(&42);

        assert_emitted!(env, topics: (symbol_short!("test"),), data: 43_u32);
    }
}
