use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

type Link = Option<Rc<Node>>;
const MAX_MODS_SIZE: usize = 5; // Path Copying Puro

#[derive(Clone, Copy, Debug)]
enum Side {
    Left,
    Right,
}

#[derive(Clone, Copy)]
enum Color {
    Red,
    Black,
}

#[derive(Clone)]
enum ModKind {
    Position(Side, Link),
    Color(Color),
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
    color: Color,
    parent: RefCell<Option<(Weak<Node>, Side)>>,
    mods: RefCell<Vec<Mod>>,
}

impl Node {
    fn get_left(&self, version: u32) -> Link {
        let mods = self.mods.borrow();
        for m in mods.iter().rev() {
            if m.version <= version {
                if let ModKind::Position(Side::Left, ref l) = m.kind {
                    return l.clone();
                }
            }
        }
        self.left.clone()
    }

    fn get_right(&self, version: u32) -> Link {
        let mods = self.mods.borrow();
        for m in mods.iter().rev() {
            if m.version <= version {
                if let ModKind::Position(Side::Right, ref r) = m.kind {
                    return r.clone();
                }
            }
        }
        self.right.clone()
    }

    fn get_value(&self, version: u32) -> i32 {
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

    fn get_color(&self, version: u32) -> Color {
        let mods = self.mods.borrow();
        for m in mods.iter().rev() {
            if m.version <= version {
                if let ModKind::Color(v) = m.kind {
                    return v;
                }
            }
        }
        self.color
    }

    // Nó que alterei + raiz
    fn update_with_node(
        self: &Rc<Self>,
        kind: ModKind,
        version: u32,
    ) -> (Rc<Node>, Option<Rc<Node>>) {
        let mut mods = self.mods.borrow_mut();

        if mods.len() < MAX_MODS_SIZE {
            mods.push(Mod { version, kind });
            return (self.clone(), None);
        }

        drop(mods);

        let mut value = self.get_value(version);
        let mut left = self.get_left(version);
        let mut right = self.get_right(version);
        let mut color = self.get_color(version);

        match kind {
            ModKind::Value(v) => value = v,
            ModKind::Position(Side::Left, l) => left = l,
            ModKind::Position(Side::Right, r) => right = r,
            ModKind::Color(c) => color = c,
        }

        let parent_info = self.parent.borrow().clone();

        let new_node = Rc::new(Node {
            value,
            color,
            left: left.clone(),
            right: right.clone(),
            parent: RefCell::new(parent_info.clone()),
            mods: RefCell::new(vec![]),
        });

        // ATUALIZAÇÃO DOS FILHOS: Crucial para o Path Copying
        if let Some(ref l) = left {
            *l.parent.borrow_mut() = Some((Rc::downgrade(&new_node), Side::Left));
        }
        if let Some(ref r) = right {
            *r.parent.borrow_mut() = Some((Rc::downgrade(&new_node), Side::Right));
        }

        if let Some((parent_weak, side)) = parent_info {
            if let Some(parent_rc) = parent_weak.upgrade() {
                let mod_to_propagate = match side {
                    Side::Left => ModKind::Position(Side::Left, Some(new_node.clone())),
                    Side::Right => ModKind::Position(Side::Right, Some(new_node.clone())),
                };
                let (_, final_root) = parent_rc.update_with_node(mod_to_propagate, version);
                return (new_node, final_root);
            }
        }
        (new_node.clone(), Some(new_node))
    }

    fn update(self: &Rc<Self>, kind: ModKind, version: u32) -> Option<Rc<Node>> {
        let (_, root) = self.update_with_node(kind, version);
        root
    }
}

fn right_rotate(y: &Rc<Node>, version: u32) -> Option<Rc<Node>> {
    let x = y.get_left(version).expect("Rotação exige filho esquerdo");
    let a = x.get_left(version);
    let b = x.get_right(version);
    let g = y.get_right(version);

    let y_value = y.get_value(version);
    let y_color = y.get_color(version);

    let x_value = x.get_value(version);
    let x_color = x.get_color(version);

    // Acumulador de raiz: se qualquer passo gerar uma nova raiz, nós a seguramos.
    let mut root = None;

    // 1. Transforma X (o que desce)
    let (nx, r) = x.update_with_node(ModKind::Value(y_value), version);
    root = r.or(root);

    let (nx, r) = nx.update_with_node(ModKind::Color(y_color), version);
    root = r.or(root);

    let (nx, r) = nx.update_with_node(ModKind::Position(Side::Left, b), version);
    root = r.or(root);

    let (nx, r) = nx.update_with_node(ModKind::Position(Side::Right, g), version);
    root = r.or(root);

    // 2. Transforma Y (o que fica no topo da subárvore)
    let (ny, r) = y.update_with_node(ModKind::Value(x_value), version);
    root = r.or(root);

    let (ny, r) = ny.update_with_node(ModKind::Color(x_color), version);
    root = r.or(root);

    let (ny, r) = ny.update_with_node(ModKind::Position(Side::Left, a), version);
    root = r.or(root);

    // 3. Conexão final
    let (_, r) = ny.update_with_node(ModKind::Position(Side::Right, Some(nx)), version);
    root = r.or(root);

    root
}

fn left_rotate(x: &Rc<Node>, version: u32) -> Option<Rc<Node>> {
    let y = x.get_right(version).expect("Rotação exige filho direito");
    let a = x.get_left(version);
    let b = y.get_left(version);
    let g = y.get_right(version);

    let x_value = x.get_value(version);
    let x_color = x.get_color(version);

    let y_value = y.get_value(version);
    let y_color = y.get_color(version);

    // Acumulador de raiz: se qualquer passo gerar uma nova raiz, nós a seguramos.
    let mut root = None;

    // 1. Transforma Y (o que era filho e agora vai "descer" para a esquerda da nova subárvore)
    // Ele assume a identidade/valor do antigo pai (x)
    let (ny, r) = y.update_with_node(ModKind::Value(x_value), version);
    root = r.or(root);

    let (ny, r) = ny.update_with_node(ModKind::Color(x_color), version);
    root = r.or(root);

    let (ny, r) = ny.update_with_node(ModKind::Position(Side::Left, a), version);
    root = r.or(root);

    let (ny, r) = ny.update_with_node(ModKind::Position(Side::Right, b), version);
    root = r.or(root);

    // 2. Transforma X (o âncora, que agora assume o valor do antigo filho Y e sobe)
    let (nx, r) = x.update_with_node(ModKind::Value(y_value), version);
    root = r.or(root);

    let (nx, r) = nx.update_with_node(ModKind::Color(y_color), version);
    root = r.or(root);

    let (nx, r) = nx.update_with_node(ModKind::Position(Side::Right, g), version);
    root = r.or(root);

    // 3. Conexão final: o novo X agora aponta para o novo Y na esquerda
    let (_, r) = nx.update_with_node(ModKind::Position(Side::Left, Some(ny)), version);
    root = r.or(root);

    root
}

// --- FUNÇÕES AUXILIARES ---

fn find_parent_for_insertion(
    root: &Link,
    value: i32,
    version: u32,
    last_parent: Option<Rc<Node>>,
) -> Option<Rc<Node>> {
    match root {
        Some(node) => {
            let v = node.get_value(version);
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
        None => last_parent,
    }
}

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

fn find_min(root: &Rc<Node>, version: u32) -> Rc<Node> {
    if let Some(left) = root.get_left(version) {
        find_min(&left, version)
    } else {
        Rc::clone(root)
    }
}

// --- OPERAÇÕES ---

fn insert(root: &Rc<Node>, value: i32, version: u32) -> Option<Rc<Node>> {
    if let Some(parent) = find_parent_for_insertion(&Some(root.clone()), value, version, None) {
        let parent_value = parent.get_value(version);
        let new_node = Rc::new(Node {
            value,
            color: Color::Red,
            left: None,
            right: None,
            parent: RefCell::new(None),
            mods: RefCell::new(vec![]),
        });

        let side = if value <= parent_value {
            Side::Left
        } else {
            Side::Right
        };
        *new_node.parent.borrow_mut() = Some((Rc::downgrade(&parent), side));
        parent.update(ModKind::Position(side, Some(new_node)), version)
    } else {
        None
    }
}

fn remove(node_to_remove: &Rc<Node>, version: u32) -> Option<Rc<Node>> {
    let left_child = node_to_remove.get_left(version);
    let right_child = node_to_remove.get_right(version);
    let parent_info = node_to_remove.parent.borrow().clone();

    // Caso de Raiz Física (Sem pai)
    if parent_info.is_none() {
        return match (left_child, right_child) {
            (None, None) => None,
            (Some(l), None) => {
                *l.parent.borrow_mut() = None;
                Some(l)
            }
            (None, Some(r)) => {
                *r.parent.borrow_mut() = None;
                Some(r)
            }
            (Some(_), Some(right_node)) => {
                let succ = find_min(&right_node, version);
                let val = succ.get_value(version);
                let root_after_val = node_to_remove.update(ModKind::Value(val), version);
                let final_root = remove(&succ, version);
                final_root.or(root_after_val)
            }
        };
    }

    let (parent_weak, side) = parent_info.unwrap();
    let parent_rc = parent_weak.upgrade().expect("Pai deve existir");

    match (left_child, right_child) {
        (None, None) => parent_rc.update(ModKind::Position(side, None), version),
        (Some(l), None) => parent_rc.update(ModKind::Position(side, Some(l)), version),
        (None, Some(r)) => parent_rc.update(ModKind::Position(side, Some(r)), version),
        (Some(_), Some(right_node)) => {
            let succ = find_min(&right_node, version);
            let val = succ.get_value(version);
            let root_after_val = node_to_remove.update(ModKind::Value(val), version);
            let final_root = remove(&succ, version);
            final_root.or(root_after_val)
        }
    }
}

// --- ESTRUTURA ---

struct PersistentStructure {
    roots: HashMap<u32, Rc<Node>>,
    current_version: u32,
}

impl PersistentStructure {
    fn new() -> Self {
        Self {
            roots: HashMap::new(),
            current_version: 0,
        }
    }

    fn insert(&mut self, value: i32) {
        let old_v = self.current_version;
        let new_v = old_v + 1;

        if let Some(root) = self.roots.get(&old_v).cloned() {
            let res = insert(&root, value, new_v);
            self.roots.insert(new_v, res.unwrap_or(root));
        } else {
            let root = Rc::new(Node {
                value,
                color: Color::Black,
                left: None,
                right: None,
                parent: RefCell::new(None),
                mods: RefCell::new(vec![]),
            });
            self.roots.insert(new_v, root);
        }
        self.current_version = new_v;
    }

    fn remove(&mut self, value: i32) {
        let old_v = self.current_version;
        let new_v = old_v + 1;

        if let Some(root) = self.roots.get(&old_v).cloned() {
            if let Some(node) = find_node(&Some(root.clone()), value, old_v) {
                if let Some(new_root) = remove(&node, new_v) {
                    self.roots.insert(new_v, new_root);
                }
            } else {
                self.roots.insert(new_v, root);
            }
        }

        self.current_version = new_v;
    }

    fn print(&self, version: u32) {
        println!("--- Versão {} ---", version);
        if let Some(root) = self.roots.get(&version) {
            Self::print_rec(&Some(root.clone()), version, 0);
        } else {
            println!("[Árvore Vazia]");
        }
    }

    fn print_rec(link: &Link, v: u32, depth: usize) {
        if let Some(n) = link {
            Self::print_rec(&n.get_right(v), v, depth + 1);
            println!("{}{}", "    ".repeat(depth), n.get_value(v));
            Self::print_rec(&n.get_left(v), v, depth + 1);
        }
    }
}

use std::io::{self, Write};

fn main() {
    let mut ps = PersistentStructure::new();

    // --- SETUP INICIAL (Árvore Exemplo) ---
    // Cria uma base para testes: 40 como raiz, 20 e 60 como filhos.
    println!("Inicializando árvore com valores: [40, 20, 60, 10, 30]");
    for v in vec![40, 20, 60, 10, 30] {
        ps.insert(v);
    }

    loop {
        println!("\n========================================");
        println!("--- Versão {} ---", ps.current_version);
        ps.print(ps.current_version);
        println!("========================================");
        println!("COMANDOS:");
        println!("  [valor][l/r] -> Rotacionar (ex: 40r, 20l)");
        println!("  v[numero]    -> Ver versão específica (ex: v2)");
        println!("  sair         -> Encerrar programa");
        print!(">> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_lowercase();

        if input == "sair" {
            println!("Saindo...");
            break;
        }

        // --- CASO 1: VISUALIZAR VERSÃO (v[n]) ---
        if input.starts_with('v') && input.len() > 1 {
            if let Ok(ver) = input[1..].parse::<u32>() {
                if ps.roots.contains_key(&ver) {
                    println!("\n--- EXPLORANDO HISTÓRICO: Versão {} ---", ver);
                    ps.print(ver);
                    println!("------------------------------------------");
                    // Opcional: Descomente a linha abaixo se quiser "teletransportar" para essa versão
                    // ps.current_version = ver;
                } else {
                    println!("Erro: A versão {} não existe no histórico.", ver);
                }
            } else {
                println!("Erro: Formato de versão inválido. Use 'v' seguido do número.");
            }
            continue;
        }

        // --- CASO 2: ROTAÇÃO ([valor][l/r]) ---
        // Extrai o último caractere para saber a direção
        if input.len() < 2 {
            println!("Comando muito curto. Tente algo como '40r'.");
            continue;
        }

        let (val_str, side) = input.split_at(input.len() - 1);

        if let Ok(val) = val_str.parse::<i32>() {
            let old_v = ps.current_version;
            let new_v = old_v + 1; // A versão SEMPRE vai subir

            if let Some(root) = ps.roots.get(&old_v) {
                // Busca o nó na versão atual
                if let Some(node_to_rotate) = find_node(&Some(root.clone()), val, old_v) {
                    let res = match side {
                        "r" => {
                            println!(
                                "Ação: Rotacionando {} à DIREITA na Versão {}...",
                                val, new_v
                            );
                            right_rotate(&node_to_rotate, new_v)
                        }
                        "l" => {
                            println!(
                                "Ação: Rotacionando {} à ESQUERDA na Versão {}...",
                                val, new_v
                            );
                            left_rotate(&node_to_rotate, new_v)
                        }
                        _ => {
                            println!("Erro: Direção inválida '{}'. Use 'l' ou 'r'.", side);
                            continue;
                        }
                    };

                    // Persistência:
                    // Se 'res' for Some, a raiz mudou (Path Copying).
                    // Se 'res' for None, a raiz física é a mesma (Fat Node ou falha na rotação).
                    let final_root = res.unwrap_or_else(|| root.clone());

                    ps.roots.insert(new_v, final_root);
                    ps.current_version = new_v;

                    println!("Sucesso! Versão {} criada e armazenada.", new_v);
                } else {
                    println!(
                        "Erro: Nó {} não encontrado na árvore (Versão {}).",
                        val, old_v
                    );
                }
            }
        } else {
            println!(
                "Entrada inválida: '{}'. Digite o valor seguido de 'l' ou 'r'.",
                input
            );
        }
    }
}
