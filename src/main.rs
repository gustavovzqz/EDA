use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

type Link = Option<Rc<Node>>;
const MAX_MODS_SIZE: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Side {
    Left,
    Right,
}

#[derive(Clone, Debug, Copy, PartialEq)]
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

fn print_root(root: &Option<Rc<Node>>, version: u32) {
    match root {
        Some(node) => {
            let color = node.get_color(version);
            println!(
                "[ROOT] Valor: {}, Cor: {:?}",
                node.get_value(version),
                color
            );
        }
        None => println!("[ROOT] A árvore está vazia (None)"),
    }
}

fn rb_insert_fixup(z: &Rc<Node>, version: u32, current_root: &Rc<Node>) -> Option<Rc<Node>> {
    let mut current = z.clone();
    let mut root_acc: Option<Rc<Node>> = None;

    // Helper para atualizar e debugar a raiz
    let mut update_root = |new_r: Option<Rc<Node>>, debug_id: u32| {
        if let Some(ref r) = new_r {
            println!("[DEBUG {}] Atualizando root_acc via update_root", debug_id);
            root_acc = Some(r.clone());
            print_root(&root_acc, version);
        }
    };

    let get_parent_info = |node: &Rc<Node>| node.parent.borrow().clone();

    while let Some((p_weak, _)) = get_parent_info(&current) {
        let p = match p_weak.upgrade() {
            Some(node) => node,
            None => break,
        };

        if p.get_color(version) == Color::Black {
            break;
        }

        let gp_info = get_parent_info(&p);

        // --- CASO 0: PAI É RAIZ ---
        if gp_info.is_none() {
            println!("entrei 1");
            let (new_p, r) = p.update_with_node(ModKind::Color(Color::Black), version);
            update_root(r.or(Some(new_p)), 1); // DEBUG 1
            break;
        }

        let (gp_weak, side_p_to_gp) = gp_info.unwrap();
        let gp = gp_weak.upgrade().expect("Avô deve estar vivo");

        if side_p_to_gp == Side::Left {
            let y = gp.get_right(version);

            if y.as_ref().map_or(Color::Black, |n| n.get_color(version)) == Color::Red {
                // --- CASO 1: RECOLORAÇÃO ---
                println!("entrei 2");
                if let Some(uncle) = y {
                    let (_, r2) = uncle.update_with_node(ModKind::Color(Color::Black), version);
                    update_root(r2, 2); // DEBUG 2
                }

                let (new_p, r1) = p.update_with_node(ModKind::Color(Color::Black), version);
                update_root(r1, 3); // DEBUG 3

                let (gp_fresh_w, _) = get_parent_info(&new_p).expect("Avô deve existir");
                let gp_fresh = gp_fresh_w.upgrade().unwrap();
                let (new_gp, r3) = gp_fresh.update_with_node(ModKind::Color(Color::Red), version);
                update_root(r3, 4); // DEBUG 4

                current = new_gp;
            } else {
                // --- CASO 2: ROTAÇÃO SIMPLES ---
                println!("entrei 3");
                if matches!(get_parent_info(&current), Some((_, Side::Right))) {
                    current = p.clone();
                    let (new_r, updated_node) = left_rotate(&current, version);
                    update_root(new_r, 5); // DEBUG 5
                    current = updated_node;
                }

                // --- CASO 3: ROTAÇÃO FINAL ---
                println!("entrei 4");
                let (p_f_w, _) = get_parent_info(&current).expect("Pai sumiu");
                let p_f = p_f_w.upgrade().unwrap();
                let (new_p, r_p) = p_f.update_with_node(ModKind::Color(Color::Black), version);
                update_root(r_p, 6); // DEBUG 6

                let (gp_f_w, _) = get_parent_info(&new_p).expect("Avô sumiu");
                let gp_f = gp_f_w.upgrade().unwrap();
                let (gp_red, r_gp) = gp_f.update_with_node(ModKind::Color(Color::Red), version);
                update_root(r_gp, 7); // DEBUG 7

                let (new_r, _) = right_rotate(&gp_red, version);
                update_root(new_r, 8); // DEBUG 8
                break;
            }
        } else {
            // --- LÓGICA ESPELHADA (P é Side::Right) ---
            println!("entrei 6");
            let y = gp.get_left(version);

            if y.as_ref().map_or(Color::Black, |n| n.get_color(version)) == Color::Red {
                // --- CASO 1 ESPELHADO ---
                if let Some(uncle) = y {
                    let (_, r2) = uncle.update_with_node(ModKind::Color(Color::Black), version);
                    update_root(r2, 9); // DEBUG 9
                }
                let (new_p, r1) = p.update_with_node(ModKind::Color(Color::Black), version);
                update_root(r1, 10); // DEBUG 10

                let (gp_f_w, _) = get_parent_info(&new_p).expect("Avô deve existir");
                let gp_f = gp_f_w.upgrade().unwrap();
                let (new_gp, r3) = gp_f.update_with_node(ModKind::Color(Color::Red), version);
                update_root(r3, 11); // DEBUG 11
                current = new_gp;
            } else {
                // --- CASO 2 ESPELHADO ---
                if matches!(get_parent_info(&current), Some((_, Side::Left))) {
                    current = p.clone();
                    let (new_r, updated_node) = right_rotate(&current, version);
                    update_root(new_r, 12); // DEBUG 12
                    current = updated_node;
                }

                // --- CASO 3 ESPELHADO ---
                let (p_f_w, _) = get_parent_info(&current).expect("Pai sumiu");
                let p_f = p_f_w.upgrade().unwrap();
                let (new_p, r_p) = p_f.update_with_node(ModKind::Color(Color::Black), version);
                update_root(r_p, 13); // DEBUG 13

                let (gp_f_w, _) = get_parent_info(&new_p).expect("Avô sumiu");
                let gp_f = gp_f_w.upgrade().unwrap();
                let (gp_red, r_gp) = gp_f.update_with_node(ModKind::Color(Color::Red), version);
                update_root(r_gp, 14); // DEBUG 14

                let (new_r, _) = left_rotate(&gp_red, version);
                update_root(new_r, 15); // DEBUG 15
                break;
            }
        }
    }

    // --- FINALIZAÇÃO ---
    let final_root = root_acc.clone().or(Some(current_root.clone())).unwrap();

    if final_root.get_color(version) == Color::Red {
        println!("[DEBUG 16] Pintura final da raiz (Red -> Black)");
        let root_after_final_paint = final_root.update(ModKind::Color(Color::Black), version);
        // Se a pintura gerou uma nova raiz (por falta de mods), retornamos ela
        root_after_final_paint.or(root_acc)
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
//

fn remove(node_to_remove: &Rc<Node>, version: u32) -> Option<Rc<Node>> {
    let color_removed = node_to_remove.get_color(version);
    let left_child = node_to_remove.get_left(version);
    let right_child = node_to_remove.get_right(version);
    let parent_info = node_to_remove.parent.borrow().clone();

    if parent_info.is_some() {
        let (parent_weak, side) = parent_info.unwrap();
        let parent_rc = parent_weak
            .upgrade()
            .expect("parent deveria sempre existir");

        match (&left_child, &right_child) {
            // Caso 1, 2 e 3: Remoção física do nó (tem pai)
            (None, None) | (Some(_), None) | (None, Some(_)) => {
                let x = left_child.or(right_child);
                let root_after_pos = parent_rc.update(ModKind::Position(side, x.clone()), version);

                if color_removed == Color::Black {
                    let fixup_root =
                        rb_delete_fixup(x, version, Some((Rc::downgrade(&parent_rc), side)));
                    fixup_root.or(root_after_pos)
                } else {
                    root_after_pos
                }
            }
            // Caso 4: Possui dois filhos (tem pai)
            (Some(_), Some(right_node)) => {
                let succ = find_min(&right_node, version);
                let color_y = succ.get_color(version);
                let val = succ.get_value(version);

                let root_after_update = node_to_remove.update(ModKind::Value(val), version);

                let (succ_parent_rc, s_side) = succ
                    .parent
                    .borrow()
                    .as_ref()
                    .map(|(weak, side)| (weak.upgrade().expect("Pai de y sumiu"), *side))
                    .expect("y (sucessor) deveria obrigatoriamente ter um pai");

                let x = succ.get_right(version);

                // TRANSPLANTAR y por x
                let root_after_removal =
                    succ_parent_rc.update(ModKind::Position(s_side, x.clone()), version);

                let final_root = root_after_removal.or(root_after_update);

                if color_y == Color::Black {
                    // O fixup começa em x, que está na posição onde o sucessor vivia
                    let fixup_root =
                        rb_delete_fixup(x, version, Some((Rc::downgrade(&succ_parent_rc), s_side)));
                    fixup_root.or(final_root)
                } else {
                    final_root
                }
            }
        }
    } else {
        // --- REMOÇÃO DA RAIZ (parent_info is None) ---
        match (&left_child, &right_child) {
            // Raiz com 0 ou 1 filho: x assume o lugar e perde o ponteiro de pai
            (None, None) | (Some(_), None) | (None, Some(_)) => {
                let x = left_child.or(right_child);

                if let Some(ref x_node) = x {
                    *x_node.parent.borrow_mut() = None;

                    if color_removed == Color::Black {
                        // Fixup na nova raiz (pai é None)
                        return rb_delete_fixup(Some(x_node.clone()), version, None).or(x);
                    }
                    return Some(x_node.clone());
                }
                None
            }
            // Raiz com 2 filhos
            (Some(_), Some(right_node)) => {
                let succ = find_min(&right_node, version);
                let color_y = succ.get_color(version);
                let val = succ.get_value(version);

                // 1. A raiz ganha o valor do sucessor
                let root_after_update = node_to_remove.update(ModKind::Value(val), version);

                // 2. Remove o sucessor de onde ele estava
                let (succ_parent_rc, s_side) = succ
                    .parent
                    .borrow()
                    .as_ref()
                    .map(|(weak, side)| (weak.upgrade().unwrap(), *side))
                    .unwrap();

                let x = succ.get_right(version);
                let root_after_removal =
                    succ_parent_rc.update(ModKind::Position(s_side, x.clone()), version);

                let final_root = root_after_removal.or(root_after_update);

                if color_y == Color::Black {
                    let fixup_root =
                        rb_delete_fixup(x, version, Some((Rc::downgrade(&succ_parent_rc), s_side)));
                    fixup_root.or(final_root)
                } else {
                    final_root
                }
            }
        }
    }
}

fn rb_delete_fixup(
    x_init: Option<Rc<Node>>,
    version: u32,
    parent_of_none: Option<(Weak<Node>, Side)>,
) -> Option<Rc<Node>> {
    // println!("[FIXUP] Iniciando fixup da versão {}", version);
    let mut current_x = x_init;
    let mut current_parent_info = parent_of_none;
    let mut root_acc: Option<Rc<Node>> = None;

    macro_rules! update_root {
        ($new_r:expr, $debug_id:expr) => {
            if let Some(ref r) = $new_r {
                println!("[DEBUG {}] Nova raiz detectada no fixup!", $debug_id);
                root_acc = Some(r.clone());
            }
        };
    }

    loop {
        // Print de estado a cada iteração
        if let Some(ref node) = current_x {
            let color = node.get_color(version);
            println!("[FIXUP] Iteração: x = {}, Cor = {:?}", node.value, color);

            // Condição de parada: se x é Vermelho ou é a Raiz
            if color == Color::Red || node.parent.borrow().is_none() {
                println!("[FIXUP] Parando: x é Vermelho ou Raiz.");
                break;
            }
        } else {
            println!("[FIXUP] Iteração: x é NIL (None)");
        }

        let (cp, side) = if let Some(ref node) = current_x {
            let info = node.parent.borrow().clone().expect("x deve ter pai");
            (info.0.upgrade().expect("Pai ativo"), info.1)
        } else if let Some((ref p_weak, s)) = current_parent_info {
            let p = p_weak.upgrade().expect("Pai de None ativo");
            println!("[FIXUP] x é NIL, pai é {}, lado {:?}", p.value, s);
            (p, s)
        } else {
            println!("[FIXUP] Sem x e sem informação de pai. Saindo.");
            break;
        };

        if side == Side::Left {
            println!("[FIXUP] Caso: x está à ESQUERDA do pai {}", cp.value);
            let mut w = cp.get_right(version).expect("Irmão deve existir");

            if w.get_color(version) == Color::Red {
                println!("[Fixup L] Caso 1: Irmão Vermelho");
                let (new_w, r1) = w.update_with_node(ModKind::Color(Color::Black), version);
                update_root!(r1, 1);

                let (p_f, _) = new_w.parent.borrow().clone().unwrap();
                let (new_p, r2) = p_f
                    .upgrade()
                    .unwrap()
                    .update_with_node(ModKind::Color(Color::Red), version);
                update_root!(r2, 2);

                let (new_root, new_p_post) = left_rotate(&new_p, version);
                update_root!(new_root, 3);
                w = new_p_post.get_right(version).expect("Novo irmão");
            }

            let w_l_c = w
                .get_left(version)
                .map_or(Color::Black, |n| n.get_color(version));
            let w_r_c = w
                .get_right(version)
                .map_or(Color::Black, |n| n.get_color(version));

            if w_l_c == Color::Black && w_r_c == Color::Black {
                println!("[Fixup L] Caso 2: Sobrinhos Pretos");
                let (new_w, r) = w.update_with_node(ModKind::Color(Color::Red), version);
                update_root!(r, 4);
                let (p_f, _) = new_w.parent.borrow().clone().unwrap();
                current_x = Some(p_f.upgrade().unwrap());
                current_parent_info = None;
            } else {
                if w_r_c == Color::Black {
                    println!("[Fixup L] Caso 3: Sobrinho oposto Preto");
                    if let Some(wl) = w.get_left(version) {
                        let (_, r) = wl.update_with_node(ModKind::Color(Color::Black), version);
                        update_root!(r, 5);
                    }
                    let (new_w, r) = w.update_with_node(ModKind::Color(Color::Red), version);
                    update_root!(r, 6);
                    let (new_root, node_below) = right_rotate(&new_w, version);
                    update_root!(new_root, 7);
                    w = node_below
                        .parent
                        .borrow()
                        .clone()
                        .unwrap()
                        .0
                        .upgrade()
                        .unwrap();
                }

                println!("[Fixup L] Caso 4: Rotação Final");
                let p_actual = if let Some(ref node) = current_x {
                    node.parent.borrow().clone().unwrap().0.upgrade().unwrap()
                } else {
                    current_parent_info.as_ref().unwrap().0.upgrade().unwrap()
                };

                let p_color = p_actual.get_color(version);
                let (new_w, r1) = w.update_with_node(ModKind::Color(p_color), version);
                update_root!(r1, 8);

                let mut final_w = new_w;
                if let Some(wr) = final_w.get_right(version) {
                    let (new_wr, r3) = wr.update_with_node(ModKind::Color(Color::Black), version);
                    update_root!(r3, 9);
                    final_w = new_wr.parent.borrow().clone().unwrap().0.upgrade().unwrap();
                }

                let (p_f_last, _) = final_w.parent.borrow().clone().unwrap();
                let (new_p_final, r2) = p_f_last
                    .upgrade()
                    .unwrap()
                    .update_with_node(ModKind::Color(Color::Black), version);
                update_root!(r2, 10);

                let (new_root, _) = left_rotate(&new_p_final, version);
                update_root!(new_root, 11);

                // IMPORTANTE: Pegamos a raiz acumulada para o encerramento
                current_x = root_acc.clone();
                break;
            }
        } else {
            println!("[FIXUP] Caso: x está à DIREITA do pai {}", cp.value);
            // O irmão (w) agora é o da ESQUERDA
            let mut w = cp.get_left(version).expect("Irmão deve existir");

            // Caso 1: Irmão é Vermelho
            if w.get_color(version) == Color::Red {
                println!("[Fixup R] Caso 1: Irmão Vermelho");
                let (new_w, r1) = w.update_with_node(ModKind::Color(Color::Black), version);
                update_root!(r1, 12);

                let (p_f, _) = new_w.parent.borrow().clone().unwrap();
                let (new_p, r2) = p_f
                    .upgrade()
                    .expect("Pai deve estar vivo")
                    .update_with_node(ModKind::Color(Color::Red), version);
                update_root!(r2, 13);

                // Inverte: nó na direita -> rotação para a DIREITA
                let (new_root, new_p_post) = right_rotate(&new_p, version);
                update_root!(new_root, 14);
                w = new_p_post.get_left(version).expect("Novo irmão");
            }

            let w_l_c = w
                .get_left(version)
                .map_or(Color::Black, |n| n.get_color(version));
            let w_r_c = w
                .get_right(version)
                .map_or(Color::Black, |n| n.get_color(version));

            // Caso 2: Ambos os sobrinhos são pretos
            if w_l_c == Color::Black && w_r_c == Color::Black {
                println!("[Fixup R] Caso 2: Sobrinhos Pretos");
                let (new_w, r) = w.update_with_node(ModKind::Color(Color::Red), version);
                update_root!(r, 15);

                let (p_f, _) = new_w.parent.borrow().clone().unwrap();
                current_x = Some(p_f.upgrade().expect("Pai deve estar vivo"));
                current_parent_info = None;
            } else {
                // Caso 3: Sobrinho da esquerda é preto (sobrinho oposto ao lado de x)
                if w_l_c == Color::Black {
                    println!("[Fixup R] Caso 3: Sobrinho oposto Preto");
                    if let Some(wr) = w.get_right(version) {
                        let (_, r) = wr.update_with_node(ModKind::Color(Color::Black), version);
                        update_root!(r, 16);
                    }
                    let (new_w, r) = w.update_with_node(ModKind::Color(Color::Red), version);
                    update_root!(r, 17);

                    let (new_root, node_below) = left_rotate(&new_w, version);
                    update_root!(new_root, 18);

                    w = node_below
                        .parent
                        .borrow()
                        .clone()
                        .unwrap()
                        .0
                        .upgrade()
                        .unwrap();
                }

                // Caso 4: Rotação Final
                println!("[Fixup R] Caso 4: Rotação Final");
                let p_actual = if let Some(ref node) = current_x {
                    node.parent.borrow().clone().unwrap().0.upgrade().unwrap()
                } else {
                    current_parent_info.as_ref().unwrap().0.upgrade().unwrap()
                };

                let p_color = p_actual.get_color(version);
                let (new_w, r1) = w.update_with_node(ModKind::Color(p_color), version);
                update_root!(r1, 19);

                let mut final_w = new_w;
                if let Some(wl) = final_w.get_left(version) {
                    let (new_wl, r3) = wl.update_with_node(ModKind::Color(Color::Black), version);
                    update_root!(r3, 20);
                    final_w = new_wl.parent.borrow().clone().unwrap().0.upgrade().unwrap();
                }

                let (p_f_last, _) = final_w.parent.borrow().clone().unwrap();
                let (new_p_final, r2) = p_f_last
                    .upgrade()
                    .unwrap()
                    .update_with_node(ModKind::Color(Color::Black), version);
                update_root!(r2, 21);

                let (new_root, _) = right_rotate(&new_p_final, version);
                update_root!(new_root, 22);

                current_x = root_acc.clone();
                break;
            }
        }
    }

    if let Some(x) = current_x {
        println!(
            "[FIXUP] Pintura final de segurança: x={} para PRETO",
            x.value
        );
        let (_, r) = x.update_with_node(ModKind::Color(Color::Black), version);
        update_root!(r.clone(), 23);

        // Retorna a raiz acumulada se houver, senão o x modificado
        return r.or(root_acc.clone());
    }

    println!("[FIXUP] Finalizado sem x final. Retornando root_acc.");
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
        let current_version = self.current_version;
        let new_version = current_version + 1;

        let root_copy = self
            .roots
            .get(&current_version)
            .cloned()
            .expect("Erro crítico: não há raiz para a versão atual.");

        let Some(node_to_remove) = find_node(&Some(root_copy.clone()), value, current_version)
        else {
            panic!("NAO HA NO PARA REMOVER");
        };

        // Se o nó é raiz (não tem pai) e não tem filhos na versão atual
        let is_root = node_to_remove.parent.borrow().is_none();
        let has_no_children = node_to_remove.get_left(current_version).is_none()
            && node_to_remove.get_right(current_version).is_none();

        if is_root && has_no_children {
            return;
        }

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

    fn sucessor(&self, x: i32, version: u32) {
        // Busca direta no HashMap conforme sua lógica
        let root = self.roots.get(&version).cloned();

        let mut best: Option<i32> = None;
        let mut curr = root;

        while let Some(node) = curr {
            if node.value > x {
                best = Some(node.value);
                curr = node.get_left(version);
            } else {
                curr = node.get_right(version);
            }
        }

        match best {
            Some(val) => println!("{}", val),
            None => println!("infinito"),
        }
    }

    fn print_imp(&self, version: u32) {
        // Busca direta no HashMap
        if let Some(root) = self.roots.get(&version).cloned() {
            let mut results = Vec::new();
            self.collect_inorder(&Some(root), version, 0, &mut results);
            println!("{}", results.join(" "));
        } else {
            // Caso a versão aponte para None (árvore vazia)
            println!("");
        }
    }

    fn collect_inorder(&self, link: &Link, v: u32, depth: u32, acc: &mut Vec<String>) {
        if let Some(node) = link {
            self.collect_inorder(&node.get_left(v), v, depth + 1, acc);

            let color_char = match node.get_color(v) {
                Color::Red => 'R',
                Color::Black => 'N',
            };

            acc.push(format!("{},{},{}", node.value, depth, color_char));

            self.collect_inorder(&node.get_right(v), v, depth + 1, acc);
        }
    }
}

// --- FUNÇÃO MAIN ---
//
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return;
    }

    let file = File::open(&args[1]).expect("Nao foi possivel abrir o arquivo");
    let reader = BufReader::new(file);

    let mut ps = PersistentStructure::new();

    for line in reader.lines() {
        let line = line.unwrap();
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        match tokens[0] {
            "INC" => {
                let val = tokens[1].parse::<i32>().unwrap();
                ps.insert(val);
            }
            "REM" => {
                let val = tokens[1].parse::<i32>().unwrap();
                ps.remove(val);
            }
            "SUC" => {
                let x = tokens[1].parse::<i32>().unwrap();
                let v = tokens[2].parse::<u32>().unwrap();
                println!("SUC {} {}", x, v);
                ps.sucessor(x, v);
            }
            "IMP" => {
                let v = tokens[1].parse::<u32>().unwrap();
                println!("IMP {}", v);
                ps.print_imp(v);
            }
            _ => {}
        }
    }
}
