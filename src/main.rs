use std::cell::RefCell;
use std::collections::HashMap;
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

    fn update(self: &Rc<Self>, kind: ModKind, version: u32) -> Option<Rc<Node>> {
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
        // Quando chegamos no None, o último nó visitado é o paii
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

fn insert(root: &Rc<Node>, value: i32, version: u32) -> Option<Rc<Node>> {
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
            parent.update(ModKind::Left(Some(new_node)), version)
        } else {
            *new_node.parent.borrow_mut() = Some((Rc::downgrade(&parent), Side::Right));
            parent.update(ModKind::Right(Some(new_node)), version)
        }
    } else {
        None
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

struct PersistentStructure {
    roots: HashMap<u32, Rc<Node>>,
    current_version: u32,
}

impl PersistentStructure {
    fn insert(&mut self, value: i32) {
        let current_version = self.current_version;

        if self.current_version == 0 {
            let root = Rc::new(Node {
                value: value,
                left: None,
                right: None,
                parent: RefCell::new(None),
                mods: RefCell::new(vec![]),
            });
            self.roots.insert(current_version, root);
        }

        let new_version = current_version + 1;

        let root_copy = self
            .roots
            .get(&current_version)
            .cloned()
            .expect("Erro crítico: não há raiz para a versão atual.");

        match insert(&root_copy, value, new_version) {
            Some(new_physical_root) => {
                self.roots.insert(new_version, new_physical_root);
            }
            None => {
                self.roots.insert(new_version, root_copy);
            }
        }

        self.current_version = new_version;
    }

    fn show_elem(&self, value: i32, version: u32) {
        let current_version = self.current_version;
        let root = self.roots.get(&current_version).cloned();

        match find_node(&root, value, version) {
            Some(_) => println!("Nó encontrado para valor {value} na versão {version}"),
            None => println!("Nó NÃO encontrado para valor {value} na versão {version}"),
        };
    }

    fn new() -> Self {
        Self {
            roots: HashMap::new(),
            current_version: 0,
        }
    }
}

fn main() {
    let mut ps = PersistentStructure::new();

    println!("========================================");
    println!("🚀 INICIANDO INSERÇÕES");
    println!("========================================");

    for i in 1..=11 {
        ps.insert(i);
        println!(
            " [+] Inserido: {:2} | Versão atual: v{}",
            i, ps.current_version
        );
    }

    println!("\n========================================");
    println!("🔍 BUSCA MULTI-VERSÃO");
    println!("========================================");

    // Loop externo: controla a versão que queremos olhar (da última para a primeira)
    for v in (0..=ps.current_version).rev() {
        println!("\n--- Lendo Árvore na Versão [v{}] ---", v);

        // Loop interno: busca elementos específicos naquela versão
        // Vou buscar de 1 a 10 para ver o que existia em cada "foto" do tempo
        print!("Elementos encontrados: ");
        for i in 1..=11 {
            // Supondo que show_elem use a versão interna ou receba uma
            // Se o seu show_elem imprime direto, ele vai aparecer aqui
            ps.show_elem(i, v);
        }
        println!();
    }

    println!("========================================");
}
