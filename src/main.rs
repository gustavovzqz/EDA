use std::cell::RefCell;
use std::rc::{Rc, Weak};

type Link = Option<Rc<Node>>;
const MAX_MODS_SIZE: usize = 5;

#[derive(Clone, Copy, Debug)]
enum Side {
    Left,
    Right,
}

#[derive(Clone)]
enum ModKind {
    Left(Link),
    Right(Link),
    Value(i32),
}

#[derive(Clone)]
struct Mod {
    version: u32,
    kind: ModKind,
}

struct Node {
    value: i32,
    left: Link,
    right: Link,
    parent: RefCell<Option<(Weak<Node>, Side)>>,
    mods: RefCell<Vec<Mod>>,
}

impl Node {
    fn get_left(self: &Rc<Self>, version: u32) -> Link {
        let mods = self.mods.borrow();
        for m in mods.iter().rev() {
            if m.version <= version {
                if let ModKind::Left(ref l) = m.kind {
                    return l.clone();
                }
            }
        }
        self.left.clone()
    }

    fn get_right(self: &Rc<Self>, version: u32) -> Link {
        let mods = self.mods.borrow();
        for m in mods.iter().rev() {
            if m.version <= version {
                if let ModKind::Right(ref r) = m.kind {
                    return r.clone();
                }
            }
        }
        self.right.clone()
    }

    fn get_value(self: &Rc<Self>, version: u32) -> i32 {
        let mods = self.mods.borrow();
        for m in mods.iter().rev() {
            if m.version <= version {
                if let ModKind::Value(v) = m.kind {
                    return v;
                }
            }
        }
        self.value
    }

    pub fn update(self: &Rc<Self>, kind: ModKind, version: u32) -> Option<Rc<Node>> {
        let mut mods = self.mods.borrow_mut();

        // Caso 1: Há espaço em MODS
        if mods.len() < MAX_MODS_SIZE {
            mods.push(Mod { version, kind });
            return None;
        }

        // Caso 2: Não há espaço em MODS
        drop(mods);

        let mut value = self.get_value(version);
        let mut left = self.get_left(version);
        let mut right = self.get_right(version);

        match kind {
            ModKind::Value(v) => value = v,
            ModKind::Left(l) => left = l,
            ModKind::Right(r) => right = r,
        }

        let parent_info = self.parent.borrow().clone();

        // 1. Crio novo nó com informações atualizadas
        let new_node = Rc::new(Node {
            value,
            left,
            right,
            parent: RefCell::new(parent_info.clone()),
            mods: RefCell::new(vec![]),
        });

        // 2. Atualizo recursivamente o PAI usando back pointer
        if let Some((parent_weak, side)) = parent_info {
            if let Some(parent_rc) = parent_weak.upgrade() {
                let mod_to_propagate = match side {
                    Side::Left => ModKind::Left(Some(new_node)),
                    Side::Right => ModKind::Right(Some(new_node)),
                };

                return parent_rc.update(mod_to_propagate, version);
            }
        }
        // 3. Caso eu não tenha pai, sou uma raiz nova.
        // Retorno a raiz para a manutenção da ED das raízes.
        Some(new_node)
    }
}

fn find_parent_for_insertion(
    root: &Link,
    value: i32,
    version: u32,
    last_parent: Option<Rc<Node>>,
) -> Option<Rc<Node>> {
    match root {
        Some(node) => {
            let v = node.get_value(version);

            // Guardamos o nó atual como o "último pai visto" e descemos
            if value <= v {
                find_parent_for_insertion(
                    &node.get_left(version),
                    value,
                    version,
                    Some(node.clone()),
                )
            } else {
                find_parent_for_insertion(
                    &node.get_right(version),
                    value,
                    version,
                    Some(node.clone()),
                )
            }
        }
        // Quando chegamos no None, o último nó visitado é o pai
        None => last_parent,
    }
}

// Busca em uma ABB
fn find_node(root: &Link, value: i32, version: u32) -> Option<Rc<Node>> {
    match root {
        Some(node) => {
            let v = node.get_value(version);

            if value < v {
                find_node(&node.get_left(version), value, version)
            } else if value > v {
                find_node(&node.get_right(version), value, version)
            } else {
                Some(node.clone())
            }
        }

        None => None,
    }
}

// TODO: Fazer retornar Option<Rc<Node>> para atualizar estrutura de dados das raizes.
fn insert(root: &Rc<Node>, value: i32, version: u32) {
    if let Some(parent) = find_parent_for_insertion(&Some(root.clone()), value, version, None) {
        let parent_value = parent.get_value(version);

        let new_node = Rc::new(Node {
            value,
            left: None,
            right: None,
            parent: RefCell::new(None),
            mods: RefCell::new(vec![]),
        });

        if value <= parent_value {
            *new_node.parent.borrow_mut() = Some((Rc::downgrade(&parent), Side::Left));
            parent.update(ModKind::Left(Some(new_node)), version);
        } else {
            *new_node.parent.borrow_mut() = Some((Rc::downgrade(&parent), Side::Right));
            parent.update(ModKind::Right(Some(new_node)), version);
        }
    }
}

fn print_tree(root: &Link, version: u32, depth: usize) {
    match root {
        Some(node) => {
            print_tree(&node.get_right(version), version, depth + 1);
            let indent = "    ".repeat(depth);
            println!("{}{}", indent, node.get_value(version));
            print_tree(&node.get_left(version), version, depth + 1);
        }
        None => {}
    }
}

fn main() {
    // 1. Setup inicial (v0)
    let root_v0 = Rc::new(Node {
        value: 0,
        left: None,
        right: None,
        parent: RefCell::new(None),
        mods: RefCell::new(vec![]),
    });

    root_v0.update(ModKind::Value(1), 1);
    root_v0.update(ModKind::Value(2), 2);
    root_v0.update(ModKind::Value(3), 3);
    root_v0.update(ModKind::Value(4), 4);
    root_v0.update(ModKind::Value(5), 5);
    root_v0.update(ModKind::Value(6), 6);
    print_tree(&Some(root_v0.clone()), 0, 0);
    print_tree(&Some(root_v0.clone()), 1, 0);
    print_tree(&Some(root_v0.clone()), 2, 0);
    print_tree(&Some(root_v0.clone()), 3, 0);
    print_tree(&Some(root_v0.clone()), 4, 0);
    print_tree(&Some(root_v0.clone()), 5, 0);
    print_tree(&Some(root_v0.clone()), 6, 0);
}
