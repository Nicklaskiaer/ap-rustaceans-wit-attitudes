use ap_project_rustaceans_wit_attitudes::drone::MyDrone;
use wg_2024::tests::{generic_chain_fragment_ack, generic_chain_fragment_drop, generic_fragment_drop, generic_fragment_forward};


#[cfg(test)]
#[test]
fn generic_fragment_forward_test() {
    generic_fragment_forward::<MyDrone>();
}
#[cfg(test)]
#[test]
fn generic_fragment_drop_test() {
    generic_fragment_drop::<MyDrone>();
}
#[cfg(test)]
#[test]
fn generic_chain_fragment_drop_test() {
    generic_chain_fragment_drop::<MyDrone>();
}
#[cfg(test)]
#[test]
fn generic_chain_fragment_ack_test() {
    generic_chain_fragment_ack::<MyDrone>();
}
