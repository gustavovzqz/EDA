use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

type Link = Option<Rc<Node>>;
const MAX_MODS_SIZE: usize = 0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Side {
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq)]
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

fn left_rotate(x: &Rc<Node>, version: u32) -> (Option<Rc<Node>>, Rc<Node>) {
    let y = x.get_right(version).expect("Rotação exige filho direito");
    let a = x.get_left(version);
    let b = y.get_left(version);
    let c = y.get_right(version);

    let x_val = x.get_value(version);
    let x_col = x.get_color(version);
    let y_val = y.get_value(version);
    let y_col = y.get_color(version);

    let mut root_acc = None;

    // 1. Criamos os novos nós (NX e NY) com os dados finais
    // NY será o filho esquerdo do novo NX
    let (ny, r) = y.update_with_node(ModKind::Value(x_val), version);
    root_acc = r.or(root_acc);
    let (ny, r) = ny.update_with_node(ModKind::Color(x_col), version);
    root_acc = r.or(root_acc);

    // NX será o novo topo
    let nx = ny
        .parent
        .borrow()
        .as_ref()
        .and_then(|(p, _)| p.upgrade())
        .unwrap();
    let (nx, r) = nx.update_with_node(ModKind::Value(y_val), version);
    root_acc = r.or(root_acc);
    let (nx, r) = nx.update_with_node(ModKind::Color(y_col), version);
    root_acc = r.or(root_acc);

    // 2. Agora FORÇAMOS as conexões de filhos no NX e NY finais
    let (nx, r) = nx.update_with_node(ModKind::Position(Side::Right, c.clone()), version);
    root_acc = r.or(root_acc);

    // NY_FINAL precisa ser reconstruído/atualizado para apontar para A e B
    let (ny_final, _) = ny.update_with_node(ModKind::Position(Side::Left, a.clone()), version);
    let (ny_final, _) =
        ny_final.update_with_node(ModKind::Position(Side::Right, b.clone()), version);

    // Reconecta NY_FINAL ao NX
    let (nx_final, r) = nx.update_with_node(
        ModKind::Position(Side::Left, Some(ny_final.clone())),
        version,
    );
    root_acc = r.or(root_acc);

    // --- A FORÇA BRUTA ---
    // Agora garantimos que todos os Weak pointers apontem para os Rcs que acabamos de criar
    println!("[Fix Manual Left] Forçando back-pointers...");

    // NY_FINAL -> NX_FINAL
    *ny_final.parent.borrow_mut() = Some((Rc::downgrade(&nx_final), Side::Left));

    // A -> NY_FINAL
    if let Some(ref node_a) = a {
        *node_a.parent.borrow_mut() = Some((Rc::downgrade(&ny_final), Side::Left));
    }
    // B -> NY_FINAL
    if let Some(ref node_b) = b {
        *node_b.parent.borrow_mut() = Some((Rc::downgrade(&ny_final), Side::Right));
    }
    // C -> NX_FINAL
    if let Some(ref node_c) = c {
        *node_c.parent.borrow_mut() = Some((Rc::downgrade(&nx_final), Side::Right));
    }

    (root_acc, ny_final)
}

fn right_rotate(y: &Rc<Node>, version: u32) -> (Option<Rc<Node>>, Rc<Node>) {
    let x = y.get_left(version).expect("Rotação exige filho esquerdo");
    let a = x.get_left(version);
    let b = x.get_right(version);
    let g = y.get_right(version);

    let y_val = y.get_value(version);
    let y_col = y.get_color(version);
    let x_val = x.get_value(version);
    let x_col = x.get_color(version);

    let mut root_acc = None;

    // 1. Criamos os novos nós NX e NY
    let (nx, r) = x.update_with_node(ModKind::Value(y_val), version);
    root_acc = r.or(root_acc);
    let (nx, r) = nx.update_with_node(ModKind::Color(y_col), version);
    root_acc = r.or(root_acc);

    let ny = nx
        .parent
        .borrow()
        .as_ref()
        .and_then(|(p, _)| p.upgrade())
        .unwrap();
    let (ny, r) = ny.update_with_node(ModKind::Value(x_val), version);
    root_acc = r.or(root_acc);
    let (ny, r) = ny.update_with_node(ModKind::Color(x_col), version);
    root_acc = r.or(root_acc);

    // 2. Montamos as posições finais
    let (ny, r) = ny.update_with_node(ModKind::Position(Side::Left, a.clone()), version);
    root_acc = r.or(root_acc);

    let (nx_final, _) = nx.update_with_node(ModKind::Position(Side::Left, b.clone()), version);
    let (nx_final, _) =
        nx_final.update_with_node(ModKind::Position(Side::Right, g.clone()), version);

    // Reconecta NX_FINAL ao NY
    let (ny_final, r) = ny.update_with_node(
        ModKind::Position(Side::Right, Some(nx_final.clone())),
        version,
    );
    root_acc = r.or(root_acc);

    // --- A FORÇA BRUTA ---
    println!("[Fix Manual Right] Forçando back-pointers...");

    // NX_FINAL -> NY_FINAL
    *nx_final.parent.borrow_mut() = Some((Rc::downgrade(&ny_final), Side::Right));

    // B -> NX_FINAL
    if let Some(ref node_b) = b {
        *node_b.parent.borrow_mut() = Some((Rc::downgrade(&nx_final), Side::Left));
    }
    // G -> NX_FINAL
    if let Some(ref node_g) = g {
        *node_g.parent.borrow_mut() = Some((Rc::downgrade(&nx_final), Side::Right));
    }
    // A -> NY_FINAL
    if let Some(ref node_a) = a {
        *node_a.parent.borrow_mut() = Some((Rc::downgrade(&ny_final), Side::Left));
    }

    (root_acc, nx_final)
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

fn rb_insert_fixup(z: &Rc<Node>, version: u32, current_root: &Rc<Node>) -> Rc<Node> {
    let mut current = z.clone();

    let mut root_acc: Option<Rc<Node>> = None;
    let get_parent_info = |node: &Rc<Node>| node.parent.borrow().clone();

    while let Some((p_weak, _)) = get_parent_info(&current) {
        let p = match p_weak.upgrade() {
            Some(node) => node,
            None => break,
        };

        if p.get_color(version) == Color::Black {
            break;
        }

        let (gp_weak, side_p_to_gp) =
            get_parent_info(&p).expect("RBT Erro: Pai vermelho exige avô");

        let gp = gp_weak.upgrade().expect("Avô deve estar vivo");

        if side_p_to_gp == Side::Left {
            let y = gp.get_right(version);

            if y.as_ref().map_or(Color::Black, |n| n.get_color(version)) == Color::Red {
                // --- CASO 1: RECOLORAÇÃO ---

                // Tio primeiro

                if let Some(uncle) = y {
                    let (_, r2) = uncle.update_with_node(ModKind::Color(Color::Black), version);

                    root_acc = r2.or(root_acc);
                }

                // Pai por último (Bússola)

                let (new_p, r1) = p.update_with_node(ModKind::Color(Color::Black), version);

                root_acc = r1.or(root_acc);

                let (gp_fresh_w, _) = get_parent_info(&new_p).expect("Avô deve existir");

                let gp_fresh = gp_fresh_w.upgrade().expect("Avô deve estar vivo");

                let (new_gp, r3) = gp_fresh.update_with_node(ModKind::Color(Color::Red), version);

                root_acc = r3.or(root_acc);

                current = new_gp;
            } else {
                // --- CASO 2: ROTAÇÃO SIMPLES ---

                if matches!(get_parent_info(&current), Some((_, Side::Right))) {
                    current = p.clone();

                    let (new_r, updated_node) = left_rotate(&current, version);

                    root_acc = new_r.or(root_acc);

                    current = updated_node;
                }

                // --- CASO 3: ROTAÇÃO FINAL ---

                let (p_f_w, _) = get_parent_info(&current).expect("Pai sumiu");

                let p_f = p_f_w.upgrade().unwrap();

                let (new_p, r_p) = p_f.update_with_node(ModKind::Color(Color::Black), version);

                root_acc = r_p.or(root_acc);

                let (gp_f_w, _) = get_parent_info(&new_p).expect("Avô sumiu");

                let gp_f = gp_f_w.upgrade().unwrap();

                let (gp_red, r_gp) = gp_f.update_with_node(ModKind::Color(Color::Red), version);

                root_acc = r_gp.or(root_acc);

                let (new_r, _) = right_rotate(&gp_red, version);

                root_acc = new_r.or(root_acc);

                break;
            }
        } else {
            // --- LÓGICA ESPELHADA (P é Side::Right) ---

            let y = gp.get_left(version);

            if y.as_ref().map_or(Color::Black, |n| n.get_color(version)) == Color::Red {
                // --- CASO 1 ESPELHADO: RECOLORAÇÃO ---

                // 1. Tio primeiro

                if let Some(uncle) = y {
                    let (_, r2) = uncle.update_with_node(ModKind::Color(Color::Black), version);

                    root_acc = r2.or(root_acc);
                }

                // 2. Pai por último (Bússola)

                let (new_p, r1) = p.update_with_node(ModKind::Color(Color::Black), version);

                root_acc = r1.or(root_acc);

                // 3. Re-ancoragem do Avô via Pai novo

                let (gp_f_w, _) = get_parent_info(&new_p).expect("Avô deve existir");

                let gp_f = gp_f_w.upgrade().expect("Avô deve estar vivo");

                // 4. Avô fica Red e subimos

                let (new_gp, r3) = gp_f.update_with_node(ModKind::Color(Color::Red), version);

                root_acc = r3.or(root_acc);

                current = new_gp;
            } else {
                // --- CASO 2 ESPELHADO ---

                if matches!(get_parent_info(&current), Some((_, Side::Left))) {
                    current = p.clone();

                    let (new_r, updated_node) = right_rotate(&current, version);

                    root_acc = new_r.or(root_acc);

                    current = updated_node;
                }

                // --- CASO 3 ESPELHADO ---

                let (p_f_w, _) = get_parent_info(&current).expect("Pai sumiu");

                let p_f = p_f_w.upgrade().expect("Pai deve estar vivo");

                let (new_p, r_p) = p_f.update_with_node(ModKind::Color(Color::Black), version);

                root_acc = r_p.or(root_acc);

                let (gp_f_w, _) = get_parent_info(&new_p).expect("Avô sumiu");

                let gp_f = gp_f_w.upgrade().expect("Avô deve estar vivo");

                let (gp_red, r_gp) = gp_f.update_with_node(ModKind::Color(Color::Red), version);

                root_acc = r_gp.or(root_acc);

                let (new_r, _) = left_rotate(&gp_red, version);

                root_acc = new_r.or(root_acc);

                break;
            }
        }
    }

    let latest_root = root_acc.unwrap_or_else(|| current_root.clone());

    latest_root
        .update(ModKind::Color(Color::Black), version)
        .unwrap_or(latest_root)
}

fn insert(root: &Rc<Node>, value: i32, version: u32) -> Option<Rc<Node>> {
    // 1. Localiza o pai para a inserção

    if let Some(parent) = find_parent_for_insertion(&Some(root.clone()), value, version, None) {
        let parent_value = parent.get_value(version);

        // 2. Cria o novo nó (sempre Vermelho)

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

        // Define o pai do novo nó

        *new_node.parent.borrow_mut() = Some((Rc::downgrade(&parent), side));

        // 3. Conecta o novo nó ao pai e captura a possível nova raiz intermediária

        // Se update retornar None, a raiz continua sendo a 'root' original

        let root_after_insertion = parent
            .update(ModKind::Position(side, Some(new_node.clone())), version)
            .unwrap_or_else(|| root.clone());

        // 4. Executa o balanceamento (Fixup)

        // O fixup agora recebe o nó novo, a versão e a raiz atualizada

        let final_root = rb_insert_fixup(&new_node, version, &root_after_insertion);

        Some(final_root)
    } else {
        // Se não encontrou pai, a árvore estava vazia (tratado no PersistentStructure)

        None
    }
}

// REMOVE

// --- AJUSTE NA FUNÇÃO DE REMOÇÃO ---

fn remove(node_to_remove: &Rc<Node>, version: u32) -> Option<Rc<Node>> {
    let color_removed = node_to_remove.get_color(version);
    let left_child = node_to_remove.get_left(version);
    let right_child = node_to_remove.get_right(version);
    let parent_info = node_to_remove.parent.borrow().clone();

    if left_child.is_some() && right_child.is_some() {
        let succ = find_min(right_child.as_ref().unwrap(), version);
        let val = succ.get_value(version);
        let root_after_val = node_to_remove.update(ModKind::Value(val), version);
        let final_root = remove(&succ, version);
        return final_root.or(root_after_val);
    }

    let x = left_child.or(right_child);

    if parent_info.is_none() {
        return if let Some(new_root) = x {
            *new_root.parent.borrow_mut() = None;
            let (new_, _) = new_root.update_with_node(ModKind::Color(Color::Black), version);
            Some(new_)
        } else {
            None
        };
    }

    let (parent_weak, side) = parent_info.clone().unwrap();

    let parent_rc = parent_weak.upgrade().expect("Pai deve existir");

    let (_, root_after_pos) =
        parent_rc.update_with_node(ModKind::Position(side, x.clone()), version);

    if color_removed == Color::Black {
        // Agora passamos o parent_info para o fixup saber onde o 'None' está pendurado

        return rb_delete_fixup(x, version, Some(parent_info.unwrap())).or(root_after_pos);
    }

    root_after_pos
}

// --- FIXUP ATUALIZADO PARA LIDAR COM NONE ---

fn rb_delete_fixup(
    x_init: Option<Rc<Node>>,

    version: u32,

    parent_of_none: Option<(Weak<Node>, Side)>,
) -> Option<Rc<Node>> {
    let mut current_x = x_init;
    let mut current_parent_info = parent_of_none;
    let mut root_acc: Option<Rc<Node>> = None;

    loop {
        // Determina quem é o pai e o lado atual

        let (cp, side) = if let Some(ref node) = current_x {
            // Se x existe, verificamos se ele é Vermelho ou Raiz (fim do fixup)

            if node.get_color(version) == Color::Red || node.parent.borrow().is_none() {
                break;
            }

            let info = node.parent.borrow().clone().expect("x deve ter pai");

            (info.0.upgrade().expect("Pai ativo"), info.1)
        } else if let Some((ref p_weak, s)) = current_parent_info {
            // Se x é None, usamos a informação do pai guardada

            (p_weak.upgrade().expect("Pai de None ativo"), s)
        } else {
            break;
        };

        if side == Side::Left {
            let mut w = cp.get_right(version).expect("Irmão deve existir");

            if w.get_color(version) == Color::Red {
                println!("[Fixup L] Entrei no Caso 1: Irmão Vermelho");

                let (new_w, r1) = w.update_with_node(ModKind::Color(Color::Black), version);

                root_acc = r1.or(root_acc);

                let (p_f, _) = new_w.parent.borrow().clone().unwrap();

                let (new_p, r2) = p_f
                    .upgrade()
                    .unwrap()
                    .update_with_node(ModKind::Color(Color::Red), version);

                root_acc = r2.or(root_acc);

                let (new_root, new_p_post) = left_rotate(&new_p, version);

                root_acc = new_root.or(root_acc);

                w = new_p_post.get_right(version).expect("Novo irmão");
            }

            let w_l_c = w
                .get_left(version)
                .map_or(Color::Black, |n| n.get_color(version));

            let w_r_c = w
                .get_right(version)
                .map_or(Color::Black, |n| n.get_color(version));

            if w_l_c == Color::Black && w_r_c == Color::Black {
                println!("[Fixup L] Entrei no Caso 2: Sobrinhos Pretos (W fica vermelho)");

                let (new_w, r) = w.update_with_node(ModKind::Color(Color::Red), version);

                root_acc = r.or(root_acc);

                let (p_f, _) = new_w.parent.borrow().clone().unwrap();

                let parent_node = p_f.upgrade().unwrap();

                current_x = Some(parent_node.clone());

                current_parent_info = None; // Agora x existe, não precisamos mais do backup
            } else {
                if w_r_c == Color::Black {
                    println!("[Fixup L] Entrei no Caso 3: Sobrinho oposto é Preto");

                    if let Some(wl) = w.get_left(version) {
                        let (new_wl, r) =
                            wl.update_with_node(ModKind::Color(Color::Black), version);
                        root_acc = r.or(root_acc);
                        w = new_wl.parent.borrow().clone().unwrap().0.upgrade().unwrap();
                    }

                    let (new_w, r) = w.update_with_node(ModKind::Color(Color::Red), version);
                    root_acc = r.or(root_acc);

                    let (new_root, node_below) = right_rotate(&new_w, version);
                    root_acc = new_root.or(root_acc);

                    w = node_below
                        .parent
                        .borrow()
                        .clone()
                        .unwrap()
                        .0
                        .upgrade()
                        .unwrap();
                }

                println!("[Fixup L] Entrei no Caso 4: Rotação Final");

                let p_actual = if let Some(ref node) = current_x {
                    node.parent.borrow().clone().unwrap().0.upgrade().unwrap()
                } else {
                    current_parent_info.as_ref().unwrap().0.upgrade().unwrap()
                };

                let p_color = p_actual.get_color(version);

                let (new_w, r1) = w.update_with_node(ModKind::Color(p_color), version);

                root_acc = r1.or(root_acc);

                let mut final_w = new_w;

                if let Some(wr) = final_w.get_right(version) {
                    let (new_wr, r3) = wr.update_with_node(ModKind::Color(Color::Black), version);

                    root_acc = r3.or(root_acc);

                    final_w = new_wr.parent.borrow().clone().unwrap().0.upgrade().unwrap();
                }

                let (p_f_last, _) = final_w.parent.borrow().clone().unwrap();

                let (new_p_final, r2) = p_f_last
                    .upgrade()
                    .unwrap()
                    .update_with_node(ModKind::Color(Color::Black), version);

                root_acc = r2.or(root_acc);

                let (new_root, _) = left_rotate(&new_p_final, version);

                root_acc = new_root.or(root_acc);

                current_x = root_acc.clone();

                break;
            }
        } else {
            // --- LÓGICA ESPELHADA (Side::Right) ---

            let mut w = cp.get_left(version).expect("Irmão deve existir");

            if w.get_color(version) == Color::Red {
                println!("[Fixup R] Entrei no Caso 1: Irmão Vermelho");

                let (new_w, r1) = w.update_with_node(ModKind::Color(Color::Black), version);

                root_acc = r1.or(root_acc);

                let (p_f, _) = new_w.parent.borrow().clone().unwrap();

                let (new_p, r2) = p_f
                    .upgrade()
                    .unwrap()
                    .update_with_node(ModKind::Color(Color::Red), version);

                root_acc = r2.or(root_acc);

                let (new_root, new_p_post) = right_rotate(&new_p, version);

                root_acc = new_root.or(root_acc);

                w = new_p_post.get_left(version).expect("Novo irmão");
            }

            let w_l_c = w
                .get_left(version)
                .map_or(Color::Black, |n| n.get_color(version));

            let w_r_c = w
                .get_right(version)
                .map_or(Color::Black, |n| n.get_color(version));

            if w_l_c == Color::Black && w_r_c == Color::Black {
                println!("[Fixup R] Entrei no Caso 2: Sobrinhos Pretos (W fica vermelho)");

                let (new_w, r) = w.update_with_node(ModKind::Color(Color::Red), version);

                root_acc = r.or(root_acc);

                let (p_f, _) = new_w.parent.borrow().clone().unwrap();

                current_x = Some(p_f.upgrade().unwrap());

                current_parent_info = None;
            } else {
                if w_l_c == Color::Black {
                    println!("[Fixup R] Entrei no Caso 3: Sobrinho oposto é Preto");

                    if let Some(wr) = w.get_right(version) {
                        let (new_wr, r) =
                            wr.update_with_node(ModKind::Color(Color::Black), version);

                        root_acc = r.or(root_acc);

                        w = new_wr.parent.borrow().clone().unwrap().0.upgrade().unwrap();
                    }

                    let (new_w, r) = w.update_with_node(ModKind::Color(Color::Red), version);

                    root_acc = r.or(root_acc);

                    let (new_root, node_below) = left_rotate(&new_w, version);

                    root_acc = new_root.or(root_acc);

                    w = node_below
                        .parent
                        .borrow()
                        .clone()
                        .unwrap()
                        .0
                        .upgrade()
                        .unwrap();
                }

                println!("[Fixup R] Entrei no Caso 4: Rotação Final");

                let p_actual = if let Some(ref node) = current_x {
                    node.parent.borrow().clone().unwrap().0.upgrade().unwrap()
                } else {
                    current_parent_info.as_ref().unwrap().0.upgrade().unwrap()
                };

                let p_color = p_actual.get_color(version);

                let (new_w, r1) = w.update_with_node(ModKind::Color(p_color), version);

                root_acc = r1.or(root_acc);

                let mut final_w = new_w;

                if let Some(wl) = final_w.get_left(version) {
                    let (new_wl, r3) = wl.update_with_node(ModKind::Color(Color::Black), version);

                    root_acc = r3.or(root_acc);

                    final_w = new_wl.parent.borrow().clone().unwrap().0.upgrade().unwrap();
                }

                let (p_f_last, _) = final_w.parent.borrow().clone().unwrap();

                let (new_p_final, r2) = p_f_last
                    .upgrade()
                    .unwrap()
                    .update_with_node(ModKind::Color(Color::Black), version);

                root_acc = r2.or(root_acc);

                let (new_root, _) = right_rotate(&new_p_final, version);

                root_acc = new_root.or(root_acc);

                current_x = root_acc.clone();

                break;
            }
        }
    }

    if let Some(x) = current_x {
        println!("[Fixup] Pintura final de X para Preto.");

        let (final_x, r) = x.update_with_node(ModKind::Color(Color::Black), version);

        return r.or(Some(final_x));
    }

    root_acc
}

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
        println!("--- Visualizando Versão {} ---", version);

        if let Some(root) = self.roots.get(&version) {
            Self::print_rec(&Some(root.clone()), version, 0);
        } else {
            println!("[Árvore Vazia]");
        }
    }

    fn print_rec(link: &Link, v: u32, depth: usize) {
        if let Some(n) = link {
            // Imprime a subárvore direita primeiro (topo do console)

            Self::print_rec(&n.get_right(v), v, depth + 1);

            // Lógica de cor

            let color_code = match n.get_color(v) {
                Color::Red => "R",

                Color::Black => "B",
            };

            // Imprime o valor com a cor ao lado: ex "40(B)"

            println!("{}{}({})", "    ".repeat(depth), n.get_value(v), color_code);

            // Imprime a subárvore esquerda

            Self::print_rec(&n.get_left(v), v, depth + 1);
        }
    }
}

fn check_rbt(root: &Link, version: u32) {
    if root.is_none() {
        println!("✅ RBT Válida (Árvore Vazia)");
        return;
    }

    let node = root.as_ref().unwrap();

    // 1. Raiz deve ser Preta
    if node.get_color(version) != Color::Black {
        println!("❌ VIOLAÇÃO: Raiz não é preta!");
    }

    let mut black_height = -1;
    let is_valid = validate_node(root, version, 0, &mut black_height);

    if is_valid {
        println!("✅ RBT Válida! (Altura Negra: {})", black_height);
    } else {
        println!("❌ VIOLAÇÃO: Propriedades da RBT quebradas!");
    }
}

fn validate_node(
    node: &Link,
    version: u32,
    current_black_count: i32,
    expected_black_height: &mut i32,
) -> bool {
    // Se chegamos no nó folha (NULL), ele é preto
    if node.is_none() {
        let final_count = current_black_count + 1; // Contando o NULL como preto
        if *expected_black_height == -1 {
            *expected_black_height = final_count;
            return true;
        }
        return final_count == *expected_black_height;
    }

    let n = node.as_ref().unwrap();
    let color = n.get_color(version);
    let left = n.get_left(version);
    let right = n.get_right(version);

    // Verificação de Cor: Nó Vermelho não pode ter filho Vermelho
    if color == Color::Red {
        if let Some(l) = &left {
            if l.get_color(version) == Color::Red {
                println!(
                    "❌ VIOLAÇÃO: Nó Vermelho {} tem filho Esquerdo Vermelho!",
                    n.get_value(version)
                );
                return false;
            }
        }
        if let Some(r) = &right {
            if r.get_color(version) == Color::Red {
                println!(
                    "❌ VIOLAÇÃO: Nó Vermelho {} tem filho Direito Vermelho!",
                    n.get_value(version)
                );
                return false;
            }
        }
    }

    // Verificação de BST: Valor esquerda < valor atual < valor direita
    if let Some(l) = &left {
        if l.get_value(version) > n.get_value(version) {
            println!(
                "❌ VIOLAÇÃO BST: {} à esquerda de {}",
                l.get_value(version),
                n.get_value(version)
            );
            return false;
        }
    }
    if let Some(r) = &right {
        if r.get_value(version) < n.get_value(version) {
            println!(
                "❌ VIOLAÇÃO BST: {} à direita de {}",
                r.get_value(version),
                n.get_value(version)
            );
            return false;
        }
    }

    // Atualiza contagem de pretos para o próximo nível
    let next_black_count = if color == Color::Black {
        current_black_count + 1
    } else {
        current_black_count
    };

    // Valida recursivamente
    validate_node(&left, version, next_black_count, expected_black_height)
        && validate_node(&right, version, next_black_count, expected_black_height)
}

use std::io::{self, Write};
fn main() {
    let mut ps = PersistentStructure::new();

    println!("=== RBT PERSISTENTE: CLI v2.0 ===");
    println!("Comandos: <num>, r <num>, v <num>, sair");

    loop {
        print!("\nComando >> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input == "sair" {
            break;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "v" => {
                if let Ok(v) = parts[1].parse::<u32>() {
                    ps.print(v);
                    if let Some(root) = ps.roots.get(&v) {
                        check_rbt(&Some(root.clone()), v);
                    }
                }
            }
            "r" => {
                if let Ok(val) = parts[1].parse::<i32>() {
                    println!("Removendo {}...", val);
                    ps.remove(val);
                    let v = ps.current_version;
                    ps.print(v);
                    check_rbt(&Some(ps.roots.get(&v).unwrap().clone()), v);
                }
            }
            _ => {
                if let Ok(val) = parts[0].parse::<i32>() {
                    println!("Inserindo {}...", val);
                    ps.insert(val);
                    let v = ps.current_version;
                    ps.print(v);
                    check_rbt(&Some(ps.roots.get(&v).unwrap().clone()), v);
                }
            }
        }
    }
}
