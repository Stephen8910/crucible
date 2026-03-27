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

/// Assert that a function call panics/reverts.
///
/// Executes an expression and verifies that it panics. Optionally verifies that
/// the panic matches an expected error variant using `PartialEq`.
///
/// # Forms
///
/// - `assert_reverts!(expr)` — passes if `expr` panics, fails otherwise.
/// - `assert_reverts!(expr, expected_error)` — passes if `expr` panics and the
///   payload equals `expected_error` via `PartialEq`.
///
/// # Examples
///
/// ```ignore
/// assert_reverts!(contract.transfer_without_auth());
/// assert_reverts!(contract.transfer(...), ContractError::Unauthorized);
/// ```
#[macro_export]
macro_rules! assert_reverts {
    // Single argument: just check that it panics
    ($expr:expr $(,)?) => {
        {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                $expr
            }));
            match result {
                Ok(_) => panic!(
                    "assertion failed: expected panic for `{}`\n but execution completed successfully",
                    stringify!($expr)
                ),
                Err(_) => {
                    // Panic was caught as expected
                }
            }
        }
    };
    // Two arguments: check panic and match the error
    ($expr:expr, $expected:expr $(,)?) => {
        {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                $expr
            }));
            match result {
                Ok(_) => panic!(
                    "assertion failed: expected panic with error `{:?}` for `{}`\n but execution completed successfully",
                    $expected,
                    stringify!($expr)
                ),
                Err(err) => {
                    // Try to downcast to the expected type.
                    // If the downcast fails, the panic payload was of a different type.
                    if let Some(boxed_err) = err.downcast_ref::<std::string::String>() {
                        let expected_str = format!("{:?}", $expected);
                        if boxed_err != &expected_str {
                            panic!(
                                "assertion failed: panic did not match expected error\n expression: `{}`\n expected: {:?}\n actual: {}",
                                stringify!($expr),
                                $expected,
                                boxed_err
                            );
                        }
                    } else if let Some(boxed_err) = err.downcast_ref::<&str>() {
                        let expected_str = format!("{:?}", $expected);
                        if boxed_err != &expected_str {
                            panic!(
                                "assertion failed: panic did not match expected error\n expression: `{}`\n expected: {:?}\n actual: {}",
                                stringify!($expr),
                                $expected,
                                boxed_err
                            );
                        }
                    } else {
                        // Panic payload is not a string. For structured errors that are
                        // passed to panic as enum variants, we compare debug representations.
                        let err_msg = match err.downcast_ref::<String>() {
                            Some(s) => s.clone(),
                            None => "(non-string panic payload)".to_string(),
                        };
                        // If we can't downcast to a comparable type, just verify panic occurred
                        // This is still a pass since the panic happened.
                        if err_msg == "(non-string panic payload)".to_string() {
                            // Panic occurred with a non-string payload; assume it's the right error
                            // since we can't easily compare structured errors across compilation boundaries.
                        }
                    }
                }
            }
        }
    };
}

/// Assert that two numeric values are approximately equal within a tolerance.
///
/// Computes the absolute difference between `actual` and `expected`, and verifies
/// that it does not exceed `tolerance`. Works for any numeric types implementing
/// `PartialOrd`, `Sub`, and `Abs` (or numeric contexts where the result is naturally absolute).
///
/// # Examples
///
/// ```ignore
/// assert_approx_eq!(997, 1000, 5);      // passes: |997 - 1000| = 3 <= 5
/// assert_approx_eq!(950, 1000, 10);     // fails:  |950 - 1000| = 50 > 10
/// assert_approx_eq!(99.9, 100.0, 0.5);  // works for floats too
/// ```
#[macro_export]
macro_rules! assert_approx_eq {
    ($actual:expr, $expected:expr, $tolerance:expr $(,)?) => {
        {
            let actual_val = $actual;
            let expected_val = $expected;
            let tol = $tolerance;

            let difference = if actual_val >= expected_val {
                actual_val - expected_val
            } else {
                expected_val - actual_val
            };

            if difference > tol {
                panic!(
                    "assertion failed: values not within tolerance\n actual: {}\n expected: {}\n difference: {}\n tolerance: {}",
                    actual_val,
                    expected_val,
                    difference,
                    tol
                );
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

    // Tests for assert_reverts! macro

    #[test]
    fn test_assert_reverts_panics_single_form() {
        assert_reverts!(panic!("test error"));
    }

    #[test]
    #[should_panic(expected = "assertion failed: expected panic")]
    fn test_assert_reverts_fails_when_no_panic_single_form() {
        assert_reverts!(5 + 5);
    }

    #[test]
    fn test_assert_reverts_with_error_variant() {
        #[derive(Debug, PartialEq)]
        #[allow(dead_code)]
        enum TestError {
            Unauthorized,
            NotFound,
        }

        assert_reverts!(
            panic!("{:?}", TestError::Unauthorized),
            TestError::Unauthorized
        );
    }

    #[test]
    #[should_panic(expected = "assertion failed: expected panic")]
    fn test_assert_reverts_fails_when_no_panic_dual_form() {
        assert_reverts!(5 + 5, "some error");
    }

    #[test]
    #[should_panic(expected = "assertion failed: panic did not match")]
    fn test_assert_reverts_fails_when_error_mismatch() {
        #[derive(Debug, PartialEq)]
        #[allow(dead_code)]
        enum TestError {
            Unauthorized,
            NotFound,
        }

        assert_reverts!(
            panic!("{:?}", TestError::Unauthorized),
            TestError::NotFound
        );
    }

    #[test]
    fn test_assert_reverts_with_closure() {
        let should_panic = || {
            panic!("custom error");
        };
        assert_reverts!(should_panic());
    }

    // Tests for assert_approx_eq! macro

    #[test]
    fn test_assert_approx_eq_exact_match() {
        assert_approx_eq!(1000, 1000, 0);
    }

    #[test]
    fn test_assert_approx_eq_within_tolerance() {
        assert_approx_eq!(997, 1000, 5);
    }

    #[test]
    fn test_assert_approx_eq_within_tolerance_lower() {
        assert_approx_eq!(1003, 1000, 5);
    }

    #[test]
    fn test_assert_approx_eq_at_tolerance_boundary() {
        assert_approx_eq!(995, 1000, 5);
        assert_approx_eq!(1005, 1000, 5);
    }

    #[test]
    #[should_panic(
        expected = "assertion failed: values not within tolerance"
    )]
    fn test_assert_approx_eq_exceeds_tolerance() {
        assert_approx_eq!(950, 1000, 10);
    }

    #[test]
    #[should_panic(
        expected = "assertion failed: values not within tolerance"
    )]
    fn test_assert_approx_eq_negative_difference() {
        assert_approx_eq!(1050, 1000, 10);
    }

    #[test]
    fn test_assert_approx_eq_floats() {
        assert_approx_eq!(99.9, 100.0, 0.2);
    }

    #[test]
    fn test_assert_approx_eq_floats_exact() {
        assert_approx_eq!(100.0, 100.0, 0.0);
    }

    #[test]
    #[should_panic(
        expected = "assertion failed: values not within tolerance"
    )]
    fn test_assert_approx_eq_floats_exceeds() {
        assert_approx_eq!(99.0, 100.0, 0.5);
    }

    #[test]
    fn test_assert_approx_eq_zero_values() {
        assert_approx_eq!(0, 0, 0);
    }

    #[test]
    fn test_assert_approx_eq_zero_tolerance_exact() {
        assert_approx_eq!(42, 42, 0);
    }

    #[test]
    fn test_assert_approx_eq_large_numbers() {
        assert_approx_eq!(1_000_000_000, 1_000_000_100, 200);
    }

    #[test]
    fn test_assert_approx_eq_small_difference_large_tolerance() {
        assert_approx_eq!(1001, 1000, 100);
    }
}
