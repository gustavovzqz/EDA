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
    color: Color,
    left: Link,
    right: Link,
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

    fn update(self: &Rc<Self>, kind: ModKind, version: u32) -> Option<Rc<Node>> {
        let mut mods = self.mods.borrow_mut();

        // Caso 1: Há espaço em MODS
        if mods.len() < MAX_MODS_SIZE {
            mods.push(Mod {
                version,
                kind: kind.clone(),
            });
            if let ModKind::Position(side, Some(ref child)) = kind {
                *child.parent.borrow_mut() = Some((Rc::downgrade(self), side));
            }
            return None;
        }

        // Caso 2: Não há espaço em MODS
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

        // 1. Crio novo nó com informações atualizadas
        let new_node = Rc::new(Node {
            value,
            left: left.clone(),
            right: right.clone(),
            color,
            parent: RefCell::new(parent_info.clone()),
            mods: RefCell::new(vec![]),
        });

        // IMPORTANTE: Atualizar back-pointers dos filhos para apontar para o novo nó
        if let Some(ref l) = left {
            *l.parent.borrow_mut() = Some((Rc::downgrade(&new_node), Side::Left));
        }
        if let Some(ref r) = right {
            *r.parent.borrow_mut() = Some((Rc::downgrade(&new_node), Side::Right));
        }

        // 2. Atualizo recursivamente o PAI usando back pointer
        if let Some((parent_weak, side)) = parent_info {
            if let Some(parent_rc) = parent_weak.upgrade() {
                let mod_to_propagate = match side {
                    Side::Left => ModKind::Position(Side::Left, Some(new_node)),
                    Side::Right => ModKind::Position(Side::Right, Some(new_node)),
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

// ROTAÇÕES
//      p             p
//      |             |
//      y             x
//     / \    ->     / \
//    x   g   <-    a   y
//   / \               / \
//  a   b             b   g

// O problema é que eu estou fazendo múltiplas atualizações em uma mesma versão.
// Preciso tomar cuidado para não estragar as coisas.
fn left_rotate(x: &Rc<Node>, version: u32) -> Option<Rc<Node>> {
    let p_info = x.parent.borrow().clone(); // Pegamos o pai original de x
    let y = x.get_right(version)?;
    let b = y.get_left(version);

    // 1. X agora aponta para B na direita
    let root_after_x = x.update(ModKind::Position(Side::Right, b.clone()), version);

    // 3. Y agora aponta para X na esquerda
    let root_after_y = y.update(ModKind::Position(Side::Left, Some(x.clone())), version);

    // 5. O pai original de X (vovô) agora aponta para Y
    let root_after_p = if let Some((ref p_weak, side)) = p_info {
        if let Some(p_rc) = p_weak.upgrade() {
            // Atualiza o back-pointer de Y para o vovô ANTES do update
            *y.parent.borrow_mut() = Some((Rc::downgrade(&p_rc), side));
            p_rc.update(ModKind::Position(side, Some(y.clone())), version)
        } else {
            None
        }
    } else {
        // Se X era a raiz, Y agora não tem pai
        *y.parent.borrow_mut() = None;
        Some(y.clone()) // Y vira a nova raiz física
    };

    // Retorna a "mais alta" nova raiz gerada
    root_after_p.or(root_after_y).or(root_after_x)
}

fn insert(root: &Rc<Node>, value: i32, version: u32) -> Option<Rc<Node>> {
    if let Some(parent) = find_parent_for_insertion(&Some(root.clone()), value, version, None) {
        let parent_value = parent.get_value(version);

        let new_node = Rc::new(Node {
            value,
            left: None,
            right: None,
            color: Color::Red,
            parent: RefCell::new(None),
            mods: RefCell::new(vec![]),
        });

        if value <= parent_value {
            parent.update(ModKind::Position(Side::Left, Some(new_node)), version)
        } else {
            parent.update(ModKind::Position(Side::Right, Some(new_node)), version)
        }
    } else {
        None
    }
}

#[allow(dead_code)]
fn successor(root: &Link, value: i32, version: u32) -> Option<Rc<Node>> {
    let mut current = root.clone();
    let mut succ: Option<Rc<Node>> = None;

    while let Some(node) = current {
        let v = node.get_value(version);

        if value < v {
            succ = Some(node.clone());
            current = node.get_left(version);
        } else if value > v {
            current = node.get_right(version);
        } else {
            if let Some(mut right) = node.get_right(version) {
                while let Some(left) = right.get_left(version) {
                    right = left;
                }
                return Some(right);
            }

            return succ;
        }
    }

    None
}

fn find_min(root: &Rc<Node>, version: u32) -> Rc<Node> {
    if let Some(left) = root.get_left(version) {
        find_min(&left, version)
    } else {
        Rc::clone(root)
    }
}
fn remove(node_to_remove: &Rc<Node>, version: u32) -> Option<Rc<Node>> {
    let left_child = node_to_remove.get_left(version);
    let right_child = node_to_remove.get_right(version);
    let parent_info = node_to_remove.parent.borrow().clone();

    // Se estamos removendo a raiz, precisaremos atualizar a ES das raizes
    if parent_info.is_none() {
        return match (left_child, right_child) {
            (None, None) => None, // Árvore ficou vazia
            (Some(l), None) => {
                *l.parent.borrow_mut() = None; // O filho vira nova raiz
                Some(l)
            }
            (None, Some(r)) => {
                *r.parent.borrow_mut() = None; // O filho vira nova raiz
                Some(r)
            }
            (Some(_), Some(right_node)) => {
                let succ = find_min(&right_node, version);
                let root_after_update =
                    node_to_remove.update(ModKind::Value(succ.get_value(version)), version);
                let root_after_removal = remove(&succ, version);
                root_after_removal.or(root_after_update)
            }
        };
    }

    let (parent_weak, side) = parent_info.unwrap();

    let parent_rc = parent_weak
        .upgrade()
        .expect("parent deveria sempre existir");

    match (&left_child, &right_child) {
        // Caso 1: Folha -> Parent aponta para NULL agora
        (None, None) => parent_rc.update(ModKind::Position(side, None), version),
        // Caso 2: Não possui filho direito -> Pai aponta para filho esquerdo de node_to_remove
        (Some(_), None) => parent_rc.update(ModKind::Position(side, left_child), version),
        // Caso 3: Não possui filho esquerdo -> Pai aponta para filho direito de node_to_remove
        (None, Some(_)) => parent_rc.update(ModKind::Position(side, right_child), version),
        // Caso 4: Possui dois filhos
        (Some(_), Some(right_node)) => {
            let succ = find_min(&right_node, version);
            let val = succ.get_value(version);
            let root_after_val_change = node_to_remove.update(ModKind::Value(val), version);
            let root_after_succ_removal = remove(&succ, version);

            root_after_succ_removal.or(root_after_val_change)
        }
    }
}

struct PersistentStructure {
    roots: HashMap<u32, Rc<Node>>,
    current_version: u32,
}

#[allow(dead_code)]
impl PersistentStructure {
    fn insert(&mut self, value: i32) {
        let current_version = self.current_version;
        let new_version = current_version + 1;

        if !self.roots.contains_key(&current_version) {
            let root = Rc::new(Node {
                value,
                color: Color::Black,
                left: None,
                right: None,
                parent: RefCell::new(None),
                mods: RefCell::new(vec![]),
            });
            self.roots.insert(new_version, root);
        } else {
            let root_copy = self.roots.get(&current_version).cloned().unwrap();

            match insert(&root_copy, value, new_version) {
                Some(new_physical_root) => {
                    self.roots.insert(new_version, new_physical_root);
                }
                None => {
                    self.roots.insert(new_version, root_copy);
                }
            }
        }

        self.current_version = new_version;
    }
    fn remove(&mut self, value: i32) {
        let current_version = self.current_version;
        let new_version = current_version + 1;

        let root_copy = self
            .roots
            .get(&current_version)
            .cloned()
            .expect("Erro crítico: não há raiz para a versão atual.");

        let Some(node_to_remove) = find_node(&Some(root_copy.clone()), value, current_version)
        else {
            panic!("eita bixo erro ó");
        };

        match remove(&node_to_remove, new_version) {
            Some(new_physical_root) => {
                self.roots.insert(new_version, new_physical_root);
            }
            None => {
                self.roots.insert(new_version, root_copy);
            }
        }

        self.current_version = new_version;
    }

    fn search(&self, value: i32, version: u32) {
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

    pub fn print(&self, version: u32) {
        println!("--- Visualizando Árvore (Versão {}) ---", version);
        if let Some(root) = self.roots.get(&version) {
            Self::print_recursive(&Some(Rc::clone(root)), version, 0);
        } else {
            println!("Versão {} não encontrada ou árvore vazia.", version);
        }
        println!("---------------------------------------");
    }

    fn print_recursive(link: &Link, version: u32, depth: usize) {
        if let Some(node) = link {
            Self::print_recursive(&node.get_right(version), version, depth + 1);
            let indent = "    ".repeat(depth);
            println!("{}{}", indent, node.get_value(version));
            Self::print_recursive(&node.get_left(version), version, depth + 1);
        }
    }
}

// testes gerados pelo gemini
fn main() {
    let mut ps = PersistentStructure::new();
    let valores = [40, 20, 60, 10, 30, 50, 70, 5, 15, 25, 35, 45, 55, 65, 75];

    for &val in &valores {
        ps.insert(val);
        println!("[v{}] Inserido: {}", ps.current_version, val);
    }

    let v_cheia = ps.current_version; // Versão com todos os itens
    println!("\nESTADO DA ÁRVORE NA VERSÃO v{}:", v_cheia);
    ps.print(v_cheia);

    println!("\n==================================================");
    println!("🧨 FASE 2: REMOÇÃO E TESTE DE PERSISTÊNCIA");
    println!("==================================================");

    // Vamos remover alguns nós estratégicos (folhas e raízes internas)
    let para_remover = [5, 40, 75, 30];
    for &val in &para_remover {
        println!("Removendo {}...", val);
        ps.remove(val);
    }

    let v_final = ps.current_version;

    println!("\n✅ ÁRVORE ATUAL (v{} - Após Remoções):", v_final);
    ps.print(v_final);

    println!("\n🔍 VOLTANDO NO TEMPO (v{}):", v_cheia);
    // O teste real: o valor 40 foi removido na v_final, mas DEVE existir na v_cheia
    println!("Buscando valor 40 (raiz antiga) na v{}:", v_cheia);
    ps.search(40, v_cheia);

    println!("\nBuscando valor 40 na v{} (deve falhar):", v_final);
    // Aqui usamos o search atualizado para a versão final
    let root_final = ps.roots.get(&v_final).cloned();
    match find_node(&root_final, 40, v_final) {
        Some(_) => println!("❌ ERRO: O nó 40 ainda existe na v{}!", v_final),
        None => println!(
            "✅ SUCESSO: O nó 40 sumiu da v{} mas permanece no histórico!",
            v_final
        ),
    }

    println!("\n==================================================");
    println!("📊 RESUMO DO HISTÓRICO");
    println!("==================================================");
    println!("Total de versões criadas: {}", ps.current_version);
    println!(
        "Nós na raiz da v1: {}",
        ps.roots.get(&1).map_or(0, |n| n.get_value(1))
    );
    println!(
        "Nós na raiz da v{}: {}",
        v_final,
        ps.roots.get(&v_final).map_or(0, |n| n.get_value(v_final))
    );
}
