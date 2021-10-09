use wasm_bindgen::prelude::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
    RwLock
};


// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

static COUNTER: AtomicUsize = AtomicUsize::new(1);
fn get_id() -> usize { COUNTER.fetch_add(1, Ordering::Relaxed) }

// Nodes need to be clonable so that each instance points to the same data
type ValueType = Arc<RwLock<Option<String>>>;
type LinksType = Arc<RwLock<BTreeMap<String, usize>>>;
type LinkedByType = Arc<RwLock<HashSet<usize>>>;
type SubscriptionsType = Arc<RwLock<HashMap<usize, js_sys::Function>>>;
type SharedNodeStore = Arc<RwLock<HashMap<usize, Node>>>;

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct Node {
    id: usize,
    value: ValueType,
    links: LinksType,
    linked_by: LinkedByType,
    subscriptions: SubscriptionsType,
    store: SharedNodeStore
}

#[wasm_bindgen]
impl Node {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            id: 0,
            value: ValueType::default(),
            links: LinksType::default(),
            linked_by: LinkedByType::default(),
            subscriptions: SubscriptionsType::default(),
            store: SharedNodeStore::default()
        }
    }

    fn new_child(parent: &mut Node, path: String) -> usize {
        let mut linked_by = HashSet::new();
        linked_by.insert(parent.id);
        let id = get_id();
        let node = Self {
            id,
            value: ValueType::default(),
            links: LinksType::default(),
            linked_by: Arc::new(RwLock::new(linked_by)),
            subscriptions: SubscriptionsType::default(),
            store: parent.store.clone()
        };
        parent.store.write().unwrap().insert(id, node);
        parent.links.write().unwrap().insert(path, id);
        id
    }

    pub fn on(&mut self, callback: js_sys::Function) -> usize {
        let value = self.value.read().unwrap();
        if value.is_some() {
            Self::_call(&callback, &JsValue::from_serde(&value.as_ref()).unwrap());
        };

        let subscription_id = get_id();
        self.subscriptions.write().unwrap().insert(subscription_id, callback);
        subscription_id
    }

    fn get_child_id(&mut self, path: String) -> usize {
        if self.value.read().unwrap().is_some() {
            Node::new_child(self, path)
        } else {
            let existing_id = match self.links.read().unwrap().get(&path) {
                Some(node_id) => Some(*node_id),
                _ => None
            };
            match existing_id {
                Some(id) => id,
                _ => Node::new_child(&mut self.clone(), path)
            }
        }
    }

    pub fn get(&mut self, path: String) -> Node {
        let id = self.get_child_id(path);
        self.store.read().unwrap().get(&id).unwrap().clone() // wasm_bindgen doesn't deal with refs
    }

    pub fn map(&self) {

    }

    pub fn put(&mut self, value: &JsValue) {
        let str = value.into_serde().unwrap_or("asdf".to_string());
        *(self.value.write().unwrap()) = Some(str);
        *(self.links.write().unwrap()) = BTreeMap::new();
        for callback in self.subscriptions.read().unwrap().values() {
            Self::_call(callback, value);
        }
    }

    fn _call(callback: &js_sys::Function, value: &JsValue) {
        let _ = callback.call1(&JsValue::null(), value); // can the function go out of scope? remove sub on Err
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let mut gun = crate::Node::new();
        let node = gun.get("asdf".to_string());
        assert_eq!(gun.id, 0);
        assert_eq!(node.id, 1);
    }
}
