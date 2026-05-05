use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

type Link = Option<Rc<Node>>;
const MAX_MODS_SIZE: usize = 2;

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

    // Retorna (RaizFinal, NoModificadoNestaChamada)
    fn update_with_node(
        self: &Rc<Self>,
        kind: ModKind,
        version: u32,
    ) -> (Option<Rc<Node>>, Rc<Node>) {
        let mut mods = self.mods.borrow_mut();

        // Caso 1: Modificação in-place (Fat Node)
        if mods.len() < MAX_MODS_SIZE {
            mods.push(Mod {
                version,
                kind: kind.clone(),
            });

            if let ModKind::Position(side, Some(ref child)) = kind {
                *child.parent.borrow_mut() = Some((Rc::downgrade(self), side));
            }

            // Se não houve cópia, a raiz final "nova" é None (não mudou o topo)
            // O nó modificado é o próprio self
            return (None, Rc::clone(self));
        }

        // Caso 2: Cópia do Nó (Path Copying)
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
            left: left.clone(),
            right: right.clone(),
            color,
            parent: RefCell::new(parent_info.clone()),
            mods: RefCell::new(vec![]),
        });

        // Atualiza back-pointers dos filhos
        if let Some(ref l) = left {
            *l.parent.borrow_mut() = Some((Rc::downgrade(&new_node), Side::Left));
        }
        if let Some(ref r) = right {
            *r.parent.borrow_mut() = Some((Rc::downgrade(&new_node), Side::Right));
        }

        // Propagação Recursiva
        if let Some((parent_weak, side)) = parent_info {
            if let Some(parent_rc) = parent_weak.upgrade() {
                let mod_to_propagate = ModKind::Position(side, Some(Rc::clone(&new_node)));

                // CHAMADA RECURSIVA
                // O pai vai retornar (RaizFinal, NóPaiCopiado)
                let (final_root, _) = parent_rc.update_with_node(mod_to_propagate, version);

                // Importante: Se o pai retornar None na raiz, significa que a propagação parou nele.
                // Mas como nós criamos um new_node, a raiz para quem chamou originalmente
                // TEM que ser no mínimo o new_node ou o que o pai retornou.
                let root_to_return = final_root.or(Some(Rc::clone(&new_node)));

                return (root_to_return, new_node);
            }
        }

        // Se não tem pai, eu sou a raiz final e o nó modificado
        (Some(Rc::clone(&new_node)), new_node)
    }

    fn update(self: &Rc<Self>, kind: ModKind, version: u32) -> Option<Rc<Node>> {
        self.update_with_node(kind, version).0
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
fn right_rotate(y: &Rc<Node>, version: u32) -> Option<Rc<Node>> {
    let x = y.get_left(version).expect("Rotação exige filho esquerdo");
    let b = x.get_right(version);

    // 0. Capturamos quem é o dono da "vaga" onde o Y está sentado.
    let p_info = y.parent.borrow().clone();

    // 1. ALTERAR O PAI PRIMEIRO (O seu jeito)
    // Fazemos o pai apontar para o X original.
    // Por enquanto, o X ainda é o antigo, mas o Pai já sabe que o X é o novo mestre da área.
    let root_p = if let Some((ref p_weak, side)) = p_info {
        if let Some(p_rc) = p_weak.upgrade() {
            // Se houver Path Copying aqui, root_p será a nova raiz global.
            p_rc.update(ModKind::Position(side, Some(x.clone())), version)
        } else {
            None
        }
    } else {
        None
    };

    // 3. ALTERAR O Y (O "filho" que desceu)
    // Por fim, o Y recebe o B na esquerda.
    *y.parent.borrow_mut() = None;
    let (_, new_y) = y.update_with_node(ModKind::Position(Side::Left, b), version);

    // 2. ALTERAR O X (O "novo pai" de Y)
    // Agora que o Pai já aponta para o X, fazemos o X apontar para o Y na direita.
    // O final_x é a versão definitiva (com ou sem Path Copying).
    let (root_x, final_x) =
        x.update_with_node(ModKind::Position(Side::Right, Some(new_y.clone())), version);

    // --- O AJUSTE FINAL ---
    // Se o Y era a raiz, o root_p foi None. Quem manda agora é o final_x.
    if p_info.is_none() {
        *final_x.parent.borrow_mut() = None;
        return Some(final_x);
    }

    // Retornamos a raiz mais alta que foi gerada na cadeia de updates.
    root_x.or(root_p)
}

fn left_rotate(x: &Rc<Node>, version: u32) -> Option<Rc<Node>> {
    let y = x.get_right(version).expect("Rotação exige filho direito");
    let b = y.get_left(version);

    // 0. Capturamos quem é o dono da "vaga" onde o Y está sentado.
    let p_info = x.parent.borrow().clone();

    // 1. ALTERAR O PAI PRIMEIRO (O seu jeito)
    // Fazemos o pai apontar para o X original.
    // Por enquanto, o X ainda é o antigo, mas o Pai já sabe que o X é o novo mestre da área.
    let root_p = if let Some((ref p_weak, side)) = p_info {
        if let Some(p_rc) = p_weak.upgrade() {
            // Se houver Path Copying aqui, root_p será a nova raiz global.
            p_rc.update(ModKind::Position(side, Some(y.clone())), version)
        } else {
            None
        }
    } else {
        None
    };

    // 3. ALTERAR O X (O "filho" que desceu)
    // Por fim, o Y recebe o B na esquerda.
    *x.parent.borrow_mut() = None;
    let (_, new_x) = x.update_with_node(ModKind::Position(Side::Right, b), version);

    // 2. ALTERAR O Y (O "novo pai" de Y)
    // Agora que o Pai já aponta para o X, fazemos o X apontar para o Y na direita.
    // O final_x é a versão definitiva (com ou sem Path Copying).
    let (root_y, final_y) =
        y.update_with_node(ModKind::Position(Side::Left, Some(new_x.clone())), version);

    // --- O AJUSTE FINAL ---
    // Se o X era a raiz, o root_p foi None. Quem manda agora é o final_y.
    if p_info.is_none() {
        *final_y.parent.borrow_mut() = None;
        return Some(final_y);
    }

    // Retornamos a raiz mais alta que foi gerada na cadeia de updates.
    root_y.or(root_p)
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

    fn rotate_left(&mut self, value: i32) {
        let v_atual = self.current_version;
        let v_nova = v_atual + 1;
        let root_atual = self.roots.get(&v_atual).cloned();

        if let Some(node) = find_node(&root_atual, value, v_atual) {
            println!("🔄 Rotacionando {} à Esquerda...", value);

            // Chama a função externa e decide qual raiz salvar
            if let Some(nova_raiz) = left_rotate(&node, v_nova).or(root_atual) {
                self.roots.insert(v_nova, nova_raiz);
                self.current_version = v_nova;
                println!("✅ Sucesso! Versão {} criada.", v_nova);
            }
        } else {
            println!("❌ Nó {} não encontrado.", value);
        }
    }

    fn rotate_right(&mut self, value: i32) {
        let v_atual = self.current_version;
        let v_nova = v_atual + 1;
        let root_atual = self.roots.get(&v_atual).cloned();

        if let Some(node) = find_node(&root_atual, value, v_atual) {
            println!("🔄 Rotacionando {} à Direita...", value);

            // Chama a função externa e decide qual raiz salvar
            if let Some(nova_raiz) = right_rotate(&node, v_nova).or(root_atual) {
                self.roots.insert(v_nova, nova_raiz);
                self.current_version = v_nova;
                println!("✅ Sucesso! Versão {} criada.", v_nova);
            }
        } else {
            println!("❌ Nó {} não encontrado.", value);
        }
    }
}

use std::io::{self, Write};

fn main() {
    let mut ps = PersistentStructure::new();
    let valores = [40, 20, 60, 10, 30, 50, 70];
    for &val in &valores {
        ps.insert(val);
    }

    loop {
        let v_atual = ps.current_version;
        println!("\n=== MÁQUINA DO TEMPO: CONTROLE DE ROTAÇÕES ===");
        ps.print(v_atual);

        println!("\nComandos: '<valor> d' (Direita), '<valor> e' (Esquerda), 'sair'");
        print!("Digite sua manobra: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input == "sair" {
            break;
        }

        let partes: Vec<&str> = input.split_whitespace().collect();
        if partes.len() < 2 {
            println!("❌ Formato inválido!");
            continue;
        }

        let val_alvo: i32 = match partes[0].parse() {
            Ok(n) => n,
            Err(_) => {
                println!("❌ Valor inválido.");
                continue;
            }
        };

        match partes[1].to_lowercase().as_str() {
            "d" => ps.rotate_right(val_alvo),
            "e" => ps.rotate_left(val_alvo),
            _ => println!("❌ Direção desconhecida."),
        }
    }
}
