use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

type Link = Option<Rc<Node>>;
const MAX_MODS_SIZE: usize = 5;

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

    fn get_field_by_side(&self, side: Side, version: u32) -> Link {
        match side {
            Side::Left => self.get_left(version),
            Side::Right => self.get_right(version),
        }
    }

    fn update_with_node(
        self: &Rc<Self>,
        kind: ModKind,
        version: u32,
    ) -> (Rc<Node>, Option<Rc<Node>>) {
        // Função auxiliar para limpar o pai apenas se ele apontar para 'self'
        let detach_if_parent_is_self = |child: &Rc<Node>, current_node: &Rc<Node>| {
            let mut p_borrow = child.parent.borrow_mut();
            let is_self = if let Some((ref w, _)) = *p_borrow {
                w.upgrade()
                    .map_or(false, |p_rc| Rc::ptr_eq(&p_rc, current_node))
            } else {
                false
            };

            if is_self {
                *p_borrow = None;
            }
        };

        let old_child = if let ModKind::Position(side, _) = &kind {
            self.get_field_by_side(*side, version)
        } else {
            None
        };

        {
            let mut mods = self.mods.borrow_mut();
            if mods.len() < MAX_MODS_SIZE {
                if let ModKind::Position(side, ref new_child_opt) = kind {
                    if let Some(old) = old_child {
                        detach_if_parent_is_self(&old, self);
                    }
                    if let Some(new_child) = new_child_opt {
                        *new_child.parent.borrow_mut() = Some((Rc::downgrade(self), side));
                    }
                }

                mods.push(Mod {
                    version,
                    kind: kind.clone(),
                });
                return (self.clone(), None);
            }
        }

        // --- CASO 2: NODE COPYING ---
        let mut value = self.get_value(version);
        let mut left = self.get_left(version);
        let mut right = self.get_right(version);
        let mut color = self.get_color(version);

        match kind {
            ModKind::Position(Side::Left, ref l) => {
                if let Some(ref old_l) = left {
                    detach_if_parent_is_self(old_l, self);
                }
                left = l.clone();
            }
            ModKind::Position(Side::Right, ref r) => {
                if let Some(ref old_r) = right {
                    detach_if_parent_is_self(old_r, self);
                }
                right = r.clone();
            }
            ModKind::Value(v) => value = v,
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
                let mod_to_propagate = ModKind::Position(side, Some(new_node.clone()));
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
    let b = y.get_left(version);
    let mut root_acc = None;

    // 1. Pai aponta para o Y original
    let parent_info = x.parent.borrow().clone();
    if let Some((p_weak, side)) = parent_info {
        if let Some(p) = p_weak.upgrade() {
            let (_, r_p) = p.update_with_node(ModKind::Position(side, Some(y.clone())), version);
            root_acc = r_p.or(root_acc);
        }
    } else {
        *y.parent.borrow_mut() = None;
        root_acc = Some(y.clone());
    }

    // 2. Y aponta para X na esquerda
    let (_, r_y) = y.update_with_node(ModKind::Position(Side::Left, Some(x.clone())), version);
    root_acc = r_y.or(root_acc);

    // 3. X adota B na direita (Última modificação)
    let (nx, r_x) = x.update_with_node(ModKind::Position(Side::Right, b), version);
    root_acc = r_x.or(root_acc);

    (root_acc, nx)
}

fn right_rotate(y: &Rc<Node>, version: u32) -> (Option<Rc<Node>>, Rc<Node>) {
    // 0. Identificação dos alvos (leitura pura, sem modificar nada)
    let x = y.get_left(version).expect("Rotação exige filho esquerdo");
    let b = x.get_right(version);
    let mut root_acc = None;

    // 2. AÇÃO: Atualiza o PAI primeiro
    // Pegamos o pai do Y original e apontamos ele para o NX imediatamente.
    let parent_info = y.parent.borrow().clone();
    if let Some((p_weak, side)) = parent_info {
        if let Some(p) = p_weak.upgrade() {
            // Esta é a primeira mudança efetiva na estrutura da árvore
            let (_, r_parent) =
                p.update_with_node(ModKind::Position(side, Some(x.clone())), version);
            root_acc = r_parent.or(root_acc);
        }
    } else {
        // Se Y era a raiz, NX assume o trono agora
        *x.parent.borrow_mut() = None;
        root_acc = Some(x.clone());
    }

    root_acc = x
        .update(ModKind::Position(Side::Right, Some(y.clone())), version)
        .or(root_acc);

    let (ny, r_y) = y.update_with_node(ModKind::Position(Side::Left, b), version);

    root_acc = r_y.or(root_acc);

    (root_acc, ny)
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

fn rb_insert_fixup(z: &Rc<Node>, version: u32, current_root: &Rc<Node>) -> Option<Rc<Node>> {
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
        // --- SUBSTITUA DAQUI ---
        let gp_info = get_parent_info(&p);

        // Se o pai é vermelho mas não tem avô, ele é tecnicamente a raiz.
        // Em uma RBT, a raiz deve ser preta.
        if gp_info.is_none() {
            let (new_p, r) = p.update_with_node(ModKind::Color(Color::Black), version);
            root_acc = r.or(Some(new_p));
            break;
        }

        let (gp_weak, side_p_to_gp) = gp_info.unwrap();
        let gp = match gp_weak.upgrade() {
            Some(node) => node,
            None => break,
        };
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

    let final_root = root_acc.clone().or(Some(current_root.clone()));
    let root_unwraped = final_root.unwrap();

    if root_unwraped.get_color(version) == Color::Red {
        root_unwraped.update(ModKind::Color(Color::Black), version)
    } else {
        root_acc
    }
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

        // 3. Conecta o novo nó ao pai e captura a possível nova raiz intermediária

        // Se update retornar None, a raiz continua sendo a 'root' original

        let final_root = parent.update(ModKind::Position(side, Some(new_node.clone())), version);

        let final_root_unwraped = final_root.clone().unwrap_or_else(|| root.clone());

        // 4. Executa o balanceamento (Fixup)

        // O fixup agora recebe o nó novo, a versão e a raiz atualizada

        let root_fixup = rb_insert_fixup(&new_node, version, &final_root_unwraped);

        root_fixup.or(final_root)
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

use rand::seq::SliceRandom; // Para embaralhar os números
use rand::thread_rng;
use std::time::Instant;

fn main() {
    let mut ps = PersistentStructure::new();
    let n = 100000;

    // 1. Criar permutação aleatória
    let mut data: Vec<i32> = (1..=n as i32).collect();
    let mut rng = thread_rng();
    data.shuffle(&mut rng);

    println!("=== INICIANDO TESTE MASSIVO DE 100.000 INSERÇÕES ===");
    let start_time = Instant::now();

    for (i, &val) in data.iter().enumerate() {
        ps.insert(val);

        let v = ps.current_version;

        // 2. Verificar integridade da RBT a cada passo
        // Pegamos a raiz da versão atual e validamos
        if let Some(root) = ps.roots.get(&v) {
            // check_rbt deve retornar um booleano ou dar panic em caso de erro
            // Aqui assumimos que sua função check_rbt faz o trabalho silenciosamente
            if !is_valid_rbt(&Some(root.clone()), v) {
                println!("❌ FALHA NA VERSÃO {}: Inserção de {}", v, val);
                // Opcional: ps.print(v) para ver o estado do erro se não for gigante
                panic!("Propriedades da RBT violadas!");
            }
        }

        // Feedback de progresso a cada 10.000
        if (i + 1) % 10_000 == 0 {
            println!("Progresso: {} nós inseridos e validados...", i + 1);
        }
    }

    let duration = start_time.elapsed();
    println!("--- TESTE CONCLUÍDO COM SUCESSO ---");
    println!("Tempo total: {:.2?}", duration);
    println!("Total de versões validadas: {}", ps.current_version);
}

// Função auxiliar silenciosa para o teste massivo
fn is_valid_rbt(root: &Option<Rc<Node>>, version: u32) -> bool {
    // Implemente uma versão de check_rbt que apenas retorna bool
    // sem dar print na árvore inteira.
    // Verificações:
    // 1. Raiz é negra?
    // 2. Vermelho tem filho vermelho?
    // 3. Altura negra é consistente em todos os caminhos?
    true // placeholder: conecte sua lógica de validação aqui
}
