use egg::*;
use ::serde::{Deserialize, Serialize};
//use crate::{Analysis, EClass, EGraph, Id, Language, RecExpr};
pub type Constant = i32;
define_language! {
    #[derive(Serialize, Deserialize)]
    pub enum Prop {
        Bool(bool),
        "*" = And([Id; 2]),
        "!" = Not(Id),
        "@" = Root(Id),
        "+" = Or([Id; 2]),
        "&" = Concat([Id; 2]),
        Constant(Constant),
        Symbol(Symbol),
    }
}

//type EGraph = egg::EGraph<Prop, ConstantFold>;


#[derive(Serialize,Deserialize,Default,Clone)]

pub struct ConstantFold;
impl Analysis<Prop> for ConstantFold {
    type Data = Option<(Constant, Vec<egg::Id>)>;

    fn merge(&mut self, to: &mut Option<(i32, Vec<egg::Id>)>, from: Option<(i32, Vec<egg::Id>)>) -> DidMerge {
        match (to.clone(), from.clone()) {
            (Some((mut to_const, mut to_ids)), Some((from_const, from_ids))) => {
                to_const += from_const;
                to_ids.extend(from_ids);
                *to = Some((to_const, to_ids));
              //  println!("1");
                DidMerge(false, false)
            }
            (Some((mut to_const, ref mut to_ids)), None) => {
              //  println!("2 - to: ({}, {:?})", to_const, to_ids);
                DidMerge(false, false)
            }
            (None, Some(( from_const,from_ids))) => {
                *to = Some((from_const.clone(), from_ids.clone()));
             //   println!("3");
                DidMerge(false, false)
            }
            (None, None) => {
             //   println!("4");
                DidMerge(false, false)
            }
        }
    }


    fn make(egraph: & egg::EGraph<Prop, ConstantFold>, enode: &Prop) -> Self::Data {
        let result = if let Some(existing_id) = egraph.lookup(&mut enode.clone()) {
            &egraph[existing_id].data
        } else {
            &None
        };
    
        // println!("Make: {:?} -> {:?}", enode, result);
        result.clone()
    }
    // fn modify(egraph: &mut egg::EGraph<Prop, ConstantFold>, id: Id) {
    //     if let Some(c) = egraph[id].data.clone() {
    //         egraph.union_instantiations(
    //             &c.1,
    //             &c.0.to_string().parse().unwrap(),
    //             &Default::default(),
    //             "analysis".to_string(),
    //         );
    //     }
    // }
}






pub fn make_rules() -> Vec<Rewrite<Prop, ()>> {
   let mut rws: Vec<Rewrite<Prop, ()>> = vec![
    // pub fn make_rules() -> Vec<Rewrite<Prop, ()>> {
        // let mut rws: Vec<Rewrite<Prop, ()>> = vec![
// Boolean theorems of one variable (Table 2.2 pg 62)
        rewrite!("null-element1"; "(* ?b n0)" => "n0"),
        rewrite!("null-element2"; "(+ ?b (! n0))" => "(! n0)"),
        rewrite!("complements1"; "(* ?b (! ?b))" => "n0"),
        rewrite!("complements2"; "(+ ?b (! ?b))" => "(! n0)"),
        rewrite!("covering1"; "(* ?b (+ ?b ?c))" => "?b"),
        rewrite!("covering2"; "(+ ?b (* ?b ?c))" => "?b"),

        rewrite!("identity1"; "(* ?b (! n0))" => "?b"),
        rewrite!("identity2'"; "(+ ?b n0)" => "?b"),
        rewrite!("idempotency1"; "(* ?b ?b)" => "?b"),
        rewrite!("idempotency2"; "(+ ?b ?b)" => "?b"),
        rewrite!("involution1"; "(! (! ?b))" => "?b"),


        rewrite!("combining1"; "(+ (* ?b ?c) (* ?b (! ?c)))" => "?b"),
        rewrite!("combining2"; "(* (+ ?b ?c) (+ ?b (! ?c)))" => "?b"),
        
        rewrite!("consensus1"; "(+ (+ (* ?b ?c) (* (! ?b) ?d)) (* ?c ?d))" => "(+ (* ?b ?c) (* (! ?b) ?d))"),
        rewrite!("consensus2"; "(* (* (+ ?b ?c) (+ (! ?b) ?d)) (+ ?c ?d))" => "(* (+ ?b ?c) (+ (! ?b) ?d))"),
        // rewrite!("distributivity1"; "(+ (* ?b ?c) (* ?b ?d))" => "(* ?b (+ ?c ?d))"),
        // rewrite!("distributivity2"; "(* (+ ?b ?c) (+ ?b ?d))" => "(+ ?b (* ?c ?d))"),
        rewrite!("commutativity1"; "(* ?b ?c)" => "(* ?c ?b)"),
        rewrite!("commutativity2"; "(+ ?b ?c)" => "(+ ?c ?b)"),
    ];


    // rws.extend(rewrite!("commutativity1"; "(* ?b ?c)" <=> "(* ?c ?b)"));
    // rws.extend(rewrite!("commutativity2"; "(+ ?b ?c)" <=> "(+ ?c ?b)"));
    rws.extend(rewrite!("associativity1"; "(*(* ?b ?c) ?d)" <=> "(* ?b (* ?c ?d))"));
    rws.extend(rewrite!("associativity2"; "(+(+ ?b ?c) ?d)" <=> "(+ ?b (+ ?c ?d))"));
    rws.extend(rewrite!("distributivity1"; "(+ ?a (* ?b ?c))" <=> "(* (+ ?a ?b) (+ ?a ?c))"));
    rws.extend(rewrite!("distributivity2"; "(* ?a (+ ?b ?c))" <=> "(+ (* ?a ?b) (* ?a ?c))"));
    rws.extend(rewrite!("de-morgan1"; "(! (* ?b ?c))" <=> "(+ (! ?b) (! ?c))"));
    rws.extend(rewrite!("de-morgan2"; "(! (+ ?b ?c))" <=> "(* (! ?b) (! ?c))"));
    
    rws
}



pub fn rules() -> Vec<Rewrite<Prop, ConstantFold>> {
    let mut rws: Vec<Rewrite<Prop, ConstantFold>> = vec![
     // pub fn make_rules() -> Vec<Rewrite<Prop, ()>> {
         // let mut rws: Vec<Rewrite<Prop, ()>> = vec![
 // Boolean theorems of one variable (Table 2.2 pg 62)
         rewrite!("null-element1"; "(* ?b 0)" => "0"),
         rewrite!("null-element2"; "(+ ?b 1)" => "1"),
         rewrite!("complements1"; "(* ?b (! ?b))" => "0"),
         rewrite!("complements2"; "(+ ?b (! ?b))" => "1"),
         rewrite!("covering1"; "(* ?b (+ ?b ?c))" => "?b"),
         rewrite!("covering2"; "(+ ?b (* ?b ?c))" => "?b"),
 
         rewrite!("identity1"; "(* ?b 1)" => "?b"),
         rewrite!("identity2'"; "(+ ?b 0)" => "?b"),
         rewrite!("idempotency1"; "(* ?b ?b)" => "?b"),
         rewrite!("idempotency2"; "(+ ?b ?b)" => "?b"),
         rewrite!("involution1"; "(! (! ?b))" => "?b"),
 
 
         rewrite!("combining1"; "(+ (* ?b ?c) (* ?b (! ?c)))" => "?b"),
         rewrite!("combining2"; "(* (+ ?b ?c) (+ ?b (! ?c)))" => "?b"),
         
         rewrite!("consensus1"; "(+ (+ (* ?b ?c) (* (! ?b) ?d)) (* ?c ?d))" => "(+ (* ?b ?c) (* (! ?b) ?d))"),
         rewrite!("consensus2"; "(* (* (+ ?b ?c) (+ (! ?b) ?d)) (+ ?c ?d))" => "(* (+ ?b ?c) (+ (! ?b) ?d))"),
         // rewrite!("distributivity1"; "(+ (* ?b ?c) (* ?b ?d))" => "(* ?b (+ ?c ?d))"),
         // rewrite!("distributivity2"; "(* (+ ?b ?c) (+ ?b ?d))" => "(+ ?b (* ?c ?d))"),
         rewrite!("commutativity1"; "(* ?b ?c)" => "(* ?c ?b)"),
         rewrite!("commutativity2"; "(+ ?b ?c)" => "(+ ?c ?b)"),
     ];
 
 
     // rws.extend(rewrite!("commutativity1"; "(* ?b ?c)" <=> "(* ?c ?b)"));
     // rws.extend(rewrite!("commutativity2"; "(+ ?b ?c)" <=> "(+ ?c ?b)"));
     rws.extend(rewrite!("associativity1"; "(*(* ?b ?c) ?d)" <=> "(* ?b (* ?c ?d))"));
     rws.extend(rewrite!("associativity2"; "(+(+ ?b ?c) ?d)" <=> "(+ ?b (+ ?c ?d))"));
     rws.extend(rewrite!("distributivity1"; "(+ ?a (* ?b ?c))" <=> "(* (+ ?a ?b) (+ ?a ?c))"));
     rws.extend(rewrite!("distributivity2"; "(* ?a (+ ?b ?c))" <=> "(+ (* ?a ?b) (* ?a ?c))"));
     rws.extend(rewrite!("de-morgan1"; "(! (* ?b ?c))" <=> "(+ (! ?b) (! ?c))"));
     rws.extend(rewrite!("de-morgan2"; "(! (+ ?b ?c))" <=> "(* (! ?b) (! ?c))"));
     
     rws
 }
pub fn make_rules_test() -> Vec<Rewrite<Prop, ConstantFold>> {
    let mut rws: Vec<Rewrite<Prop, ConstantFold>> = vec![
        // 1 var laws
        rewrite!("idempotent 1"; "(* ?b ?b)" => "?b"),
        rewrite!("idempotent 2"; "(! (! ?b))" => "?b"),
        // rewrite!("Identity 1"; "(* ?b 1)" => "?b"),
        // rewrite!("Identity 2"; "(+ ?b 0)" => "?b"),
        // rewrite!("annihilator 1"; "(* ?b 0)" => "0"),
        // rewrite!("annihilator 2"; "(+ ?b 1)" => "1"),
        // rewrite!("complements1"; "(* ?b (! ?b))" => "0"),
        // rewrite!("complements2"; "(+ ?b (! ?b))" => "1"),

        rewrite!("Identity 1"; "(* ?b true)" => "?b"),
        rewrite!("Identity 2"; "(+ ?b false)" => "?b"),
        rewrite!("annihilator 1"; "(* ?b false)" => "false"),
        rewrite!("annihilator 2"; "(+ ?b true)" => "true"),
        rewrite!("complements1"; "(* ?b (! ?b))" => "false"),
        rewrite!("complements2"; "(+ ?b (! ?b))" => "true"),
        // 2 distinct var laws
        // rewrite!("Absorption1"; "(* ?b (+ ?b ?c))" => "?b"),
        // rewrite!("Absorption2"; "(+ ?b (* ?b ?c))" => "?b"),
        // rewrite!("combining1"; "(+ (* ?b ?c) (* ?b (! ?c)))" => "?b"),
        // rewrite!("combining2"; "(* (+ ?b ?c) (+ ?b (! ?c)))" => "?b"), 
        rewrite!("commutativity1"; "(* ?b ?c)" => "(* ?c ?b)"),
        rewrite!("commutativity2"; "(+ ?b ?c)" => "(+ ?c ?b)"),
        // rewrite!("de-morgan1"; "(! (* ?b ?c))" => "(+ (! ?b) (! ?c))"),
        // rewrite!("de-morgan2"; "(! (+ ?b ?c))" => "(* (! ?b) (! ?c))"),
        // 3 distinct var laws //dual?
        // rewrite!("associativity1"; "(*(* ?b ?c) ?d)" => "(* ?b (* ?c ?d))"),
        // rewrite!("associativity2"; "(+(+ ?b ?c) ?d)" => "(+ ?b (+ ?c ?d))"),
        rewrite!("distributivity1"; "(+ ?a (* ?b ?c))" => "(* (+ ?a ?b) (+ ?a ?c))"),
        rewrite!("distributivity2"; "(* ?a (+ ?b ?c))" => "(+ (* ?a ?b) (* ?a ?c))"),

];

// 2 distinct var laws
// rws.extend(rewrite!("commutativity1"; "(* ?b ?c)" <=> "(* ?c ?b)"));
// rws.extend(rewrite!("commutativity2"; "(+ ?b ?c)" <=> "(+ ?c ?b)"));


// 3 distinct var laws
rws.extend(rewrite!("associativity1"; "(*(* ?b ?c) ?d)" <=> "(* ?b (* ?c ?d))"));
rws.extend(rewrite!("associativity2"; "(+(+ ?b ?c) ?d)" <=> "(+ ?b (+ ?c ?d))"));
rws.extend(rewrite!("de-morgan1"; "(! (* ?b ?c))" <=> "(+ (! ?b) (! ?c))"));
rws.extend(rewrite!("de-morgan2"; "(! (+ ?b ?c))" <=> "(* (! ?b) (! ?c))"));
// rws.extend(rewrite!("de-morgan1"; "(! (* ?b ?c))" <=> "(+ (! ?b) (! ?c))"));
// rws.extend(rewrite!("de-morgan2"; "(! (+ ?b ?c))" <=> "(* (! ?b) (! ?c))"));
rws
}


pub fn make_rules_or_replace() -> Vec<Rewrite<Prop, ()>> {
    vec![
        rewrite!("involution1"; "(! (! ?b))" => "?b"),
    ]
}
// pub fn make_rules() -> Vec<Rewrite<Prop, ConstantFold>> {
//     let mut rws: Vec<Rewrite<Prop, ConstantFold>> = vec![
//         // Boolean theorems of one variable (Table 2.2 pg 62)
//         rewrite!("null-element1"; "(* ?b 0)" => "0"),
//         rewrite!("null-element2"; "(+ ?b 1)" => "1"),
//         rewrite!("complements1"; "(* ?b (! ?b))" => "0"),
//         rewrite!("complements2"; "(+ ?b (! ?b))" => "1"),
//         rewrite!("covering1"; "(* ?b (+ ?b ?c))" => "?b"),
//         rewrite!("covering2"; "(+ ?b (* ?b ?c))" => "?b"),
//         rewrite!("combining1"; "(+ (* ?b ?c) (* ?b (! ?c)))" => "?b"),
//         rewrite!("combining2"; "(* (+ ?b ?c) (+ ?b (! ?c)))" => "?b"),
//         rewrite!("distributivity3"; "(* ?a (+ ?b ?c))" => "(+ (* ?a ?b) (* ?a ?c))"),
//         //rewrite!("q"; "(+ ?a (! ?a))"   =>    "1"                   ) ,
//         //rewrite!("null-element1"; "(* ?b 0)" => "0"),
//         //rewrite!("null-element2"; "(+ ?b 1)" => "1"),
//         //rewrite!("complements1"; "(* ?b (! ?b))" => "0"),
//         //rewrite!("identity1"; "(* ?b 1)" => "?b"),
//         //rewrite!("identity2'"; "(+ ?b 0)" => "?b"),
//         //rewrite!("involution1"; "(! (! ?a))"      =>       "?a"                     ),
//         //rewrite!("associativity2"; "(+ ?a (+ ?b ?c))"=> "(+ (+ ?a ?b) ?c)"       ),
//         // rewrite!("d"; "(* ?a (+ ?b ?c))"=> "(+ (* ?a ?b) (* ?a ?c))"),
//         // rewrite!("e"; "(+ ?a (* ?b ?c))"=> "(* (+ ?a ?b) (+ ?a ?c))"),
//         // //rewrite!("f"; "(+ ?a ?b)"       =>        "(+ ?b ?a)"              ),
//         // // rewrite!("r"; "(* ?a ?b)"       =>        "(* ?b ?a)"              ),
//         // rewrite!("th1"; "(+ ?x (* ?x ?y))" => "?x"),
//         // // Theorem 2: X + !X · Y = X + Y
//         // rewrite!("th2"; "(+ ?x (* (! ?x) ?y))" => "(+ ?x ?y)"),
//         // // Theorem 3: X · Y + !X · Z + Y · Z = X · Y + !X · Z
//         // rewrite!("th3"; "(+ (* ?x ?y) (+ (* (! ?x) ?z) (* ?y ?z)))" => "(+ (* ?x ?y) (* (! ?x) ?z))"),
//         // // Theorem 4: X(X + Y) = X
//         // rewrite!("th4"; "(* ?x (+ ?x ?y))" => "?x"),
//         // // Theorem 5: X(!X + Y) = X · Y
//         // rewrite!("th5"; "(* ?x (+ (! ?x) ?y))" => "(* ?x ?y)"),
//         // // Theorem 6: (X + Y)(X + !Y) = X
//         // rewrite!("th6"; "(* (+ ?x ?y) (+ ?x (! ?y)))" => "?x"),
//         // // Theorem 7: (X + Y)(!X + Z) = X · Z + !X · Y
//         // rewrite!("th7"; "(* (+ ?x ?y) (+ (! ?x) ?z))" => "(+ (* ?x ?z) (* (! ?x) ?y))"),
//         // // Theorem 8: (X + Y)(!X + Z)(Y + Z) = (X + Y)(!X + Z)
//         // rewrite!("th8"; "(* (+ ?x ?y) (* (+ (! ?x) ?z) (+ ?y ?z)))" => "(* (+ ?x ?y) (+ (! ?x) ?z))"),
//     ];

//     rws.extend(rewrite!("identity1"; "(* ?b 1)" <=> "?b"));
//     rws.extend(rewrite!("identity2'"; "(+ ?b 0)" <=> "?b"));
//     rws.extend(rewrite!("idempotency1"; "(* ?b ?b)" <=> "?b"));
//     rws.extend(rewrite!("idempotency2"; "(+ ?b ?b)" <=> "?b"));
//     rws.extend(rewrite!("involution1"; "(! (! ?b))" <=> "?b"));
//     rws.extend(rewrite!("commutativity1"; "(* ?b ?c)" <=> "(* ?c ?b)"));
//     rws.extend(rewrite!("commutativity2"; "(+ ?b ?c)" <=> "(+ ?c ?b)"));
//     rws.extend(rewrite!("associativity1"; "(*(* ?b ?c) ?d)" <=> "(* ?b (* ?c ?d))"));
//     rws.extend(rewrite!("associativity2"; "(+(+ ?b ?c) ?d)" <=> "(+ ?b (+ ?c ?d))"));
//     rws.extend(rewrite!("distributivity1"; "(+ (* ?b ?c) (* ?b ?d))" <=> "(* ?b (+ ?c ?d))"));
//     rws.extend(rewrite!("distributivity2"; "(* (+ ?b ?c) (+ ?b ?d))" <=> "(+ ?b (* ?c ?d))"));
   
//     rws.extend(rewrite!("consensus1"; "(+ (+ (* ?b ?c) (* (! ?b) ?d)) (* ?c ?d))" <=> "(+ (* ?b ?c) (* (! ?b) ?d))"));
//     rws.extend(rewrite!("consensus2"; "(* (* (+ ?b ?c) (+ (! ?b) ?d)) (+ ?c ?d))" <=> "(* (+ ?b ?c) (+ (! ?b) ?d))"));
//     rws.extend(rewrite!("de-morgan1"; "(! (* ?b ?c))" <=> "(+ (! ?b) (! ?c))"));
//     rws.extend(rewrite!("de-morgan2"; "(! (+ ?b ?c))" <=> "(* (! ?b) (! ?c))"));
//     rws
// }




//test_rules_most
// pub fn make_rules() -> Vec<Rewrite<Prop, ConstantFold>> {
//     vec![
//         //version 1
//         //rewrite!("a"; "(-> ?a ?b)"      =>       "(+ (! ?a) ?b)"          ),
//         rewrite!("q"; "(+ ?a (! ?a))"   =>    "1"                   ) ,
//         rewrite!("null-element1"; "(* ?b 0)" => "0"),
//         rewrite!("null-element2"; "(+ ?b 1)" => "1"),
//         rewrite!("complements1"; "(* ?b (! ?b))" => "0"),
//         rewrite!("identity1"; "(* ?b 1)" => "?b"),
//         rewrite!("identity2'"; "(+ ?b 0)" => "?b"),


//         rewrite!("involution1"; "(! (! ?a))"      =>       "?a"                     ),
//         rewrite!("associativity2"; "(+ ?a (+ ?b ?c))"=> "(+ (+ ?a ?b) ?c)"       ),
//         rewrite!("d"; "(* ?a (+ ?b ?c))"=> "(+ (* ?a ?b) (* ?a ?c))"),
//         rewrite!("e"; "(+ ?a (* ?b ?c))"=> "(* (+ ?a ?b) (+ ?a ?c))"),
//         rewrite!("f"; "(+ ?a ?b)"       =>        "(+ ?b ?a)"              ),
//         rewrite!("r"; "(* ?a ?b)"       =>        "(* ?b ?a)"              ),


//         rewrite!("th1"; "(+ ?x (* ?x ?y))" => "?x"),
//         // Theorem 2: X + !X · Y = X + Y
//         rewrite!("th2"; "(+ ?x (* (! ?x) ?y))" => "(+ ?x ?y)"),
//         // Theorem 3: X · Y + !X · Z + Y · Z = X · Y + !X · Z
//         rewrite!("th3"; "(+ (* ?x ?y) (+ (* (! ?x) ?z) (* ?y ?z)))" => "(+ (* ?x ?y) (* (! ?x) ?z))"),
//         // Theorem 4: X(X + Y) = X
//         rewrite!("th4"; "(* ?x (+ ?x ?y))" => "?x"),
//         // Theorem 5: X(!X + Y) = X · Y
//         rewrite!("th5"; "(* ?x (+ (! ?x) ?y))" => "(* ?x ?y)"),
//         // Theorem 6: (X + Y)(X + !Y) = X
//         rewrite!("th6"; "(* (+ ?x ?y) (+ ?x (! ?y)))" => "?x"),
//         // Theorem 7: (X + Y)(!X + Z) = X · Z + !X · Y
//         rewrite!("th7"; "(* (+ ?x ?y) (+ (! ?x) ?z))" => "(+ (* ?x ?z) (* (! ?x) ?y))"),
//         // Theorem 8: (X + Y)(!X + Z)(Y + Z) = (X + Y)(!X + Z)
//         rewrite!("th8"; "(* (+ ?x ?y) (* (+ (! ?x) ?z) (+ ?y ?z)))" => "(* (+ ?x ?y) (+ (! ?x) ?z))"),
//         //version2
//     //     rewrite!("identity"; "(* ?b true)" => "?b"),
//     //     rewrite!("identity'"; "(+ ?b false)" => "?b"),
//     //     rewrite!("null-element"; "(* ?b false)" => "false"),
//     //   // rewrite!("null-element"; "false" => "(* ?b false)"),
//     //     rewrite!("null-element'"; "(+ ?b true)" => "true"),
//     //   // rewrite!("null-element'"; "true"=> "(+ ?b true)"),
//     //     rewrite!("idempotency"; "(* ?b ?b)" => "?b"),
//     //     rewrite!("idempotency'"; "(+ ?b ?b)" => "?b"),
//     //     rewrite!("involution"; "(! (! ?b))" => "?b"),
//     //     rewrite!("complements"; "(* ?b (! ?b))" => "false"),
//     //     rewrite!("complements'"; "(+ ?b (! ?b))" => "true"),
//     //     // Boolean theorems of several variables (Table 2.3 pg 63)
//     //     rewrite!("commutativity"; "(* ?b ?c)" => "(* ?c ?b)"),
//     //     rewrite!("commutativity'"; "(+ ?b ?c)" => "(+ ?c ?b)"),
//     //     rewrite!("associativity"; "(*(* ?b ?c) ?d)" => "(* ?b (* ?c ?d))"),
//     //     rewrite!("associativity'"; "(+(+ ?b ?c) ?d)" => "(+ ?b (+ ?c ?d))"),
//     //     rewrite!("distributivity"; "(+ (* ?b ?c) (* ?b ?d))" => "(* ?b (+ ?c ?d))"),
//     //     rewrite!("distributivity'"; "(* (+ ?b ?c) (+ ?b ?d))" => "(+ ?b (* ?c ?d))"),
//     //     rewrite!("covering"; "(* ?b (+ ?b ?c))" => "?b"),
//     //     rewrite!("covering'"; "(+ ?b (* ?b ?c))" => "?b"),
//     //     rewrite!("combining"; "(+ (* ?b ?c) (* ?b (! ?c)))" => "?b"),
//     //     rewrite!("combining'"; "(* (+ ?b ?c) (+ ?b (! ?c)))" => "?b"),
//     //     rewrite!("consensus"; "(+ (+ (* ?b ?c) (* (! ?b) ?d)) (* ?c ?d))" => "(+ (* ?b ?c) (* (! ?b) ?d))"),
//     //     rewrite!("consensus'"; "(* (* (+ ?b ?c) (+ (! ?b) ?d)) (+ ?c ?d))" => "(* (+ ?b ?c) (+ (! ?b) ?d))"),
//     //     rewrite!("de-morgan"; "(! (* ?b ?c))" => "(+ (! ?b) (! ?c))"),
//     //     rewrite!("de-morgan'"; "(! (+ ?b ?c))" => "(* (! ?b) (! ?c))"),
//     //  version3 
//     // rewrite!("a"; "(-> ?a ?b)"      =>       "(+ (! ?a) ?b)"          ),
//     // rewrite!("b"; "(! (! ?a))"      =>       "?a"                     ),
//     // rewrite!("c"; "(+ ?a (+ ?b ?c))"=> "(+ (+ ?a ?b) ?c)"       ),
//     // rewrite!("d"; "(* ?a (+ ?b ?c))"=> "(+ (* ?a ?b) (* ?a ?c))"),
//     // rewrite!("e"; "(+ ?a (* ?b ?c))"=> "(* (+ ?a ?b) (+ ?a ?c))"),
//     // rewrite!("f"; "(+ ?a ?b)"       =>        "(+ ?b ?a)"              ),
//     // rewrite!("r"; "(* ?a ?b)"       =>        "(* ?b ?a)"              ),
//     // //rewrite!("q"; "(+ ?a (! ?a))"   =>    "true"                   ) ,
//     // //rewrite!("s"; "(+ ?a true)"     =>         "true"                ) ,
//     // rewrite!("g"; "(* ?a true)"     =>         "?a"                  ),
//     // rewrite!("y"; "(-> ?a ?b)"      =>    "(-> (! ?b) (! ?a))"     ),
//     // rewrite!("th1"; "(+ ?x (* ?x ?y))" => "?x"),
//     // // Theorem 2: X + !X · Y = X + Y
//     // rewrite!("th2"; "(+ ?x (* (! ?x) ?y))" => "(+ ?x ?y)"),
//     // // Theorem 3: X · Y + !X · Z + Y · Z = X · Y + !X · Z
//     // rewrite!("th3"; "(+ (* ?x ?y) (+ (* (! ?x) ?z) (* ?y ?z)))" => "(+ (* ?x ?y) (* (! ?x) ?z))"),
//     // // Theorem 4: X(X + Y) = X
//     // rewrite!("th4"; "(* ?x (+ ?x ?y))" => "?x"),
//     // // Theorem 5: X(!X + Y) = X · Y
//     // rewrite!("th5"; "(* ?x (+ (! ?x) ?y))" => "(* ?x ?y)"),
//     // // Theorem 6: (X + Y)(X + !Y) = X
//     // rewrite!("th6"; "(* (+ ?x ?y) (+ ?x (! ?y)))" => "?x"),
//     // // Theorem 7: (X + Y)(!X + Z) = X · Z + !X · Y
//     // rewrite!("th7"; "(* (+ ?x ?y) (+ (! ?x) ?z))" => "(+ (* ?x ?z) (* (! ?x) ?y))"),
//     // // Theorem 8: (X + Y)(!X + Z)(Y + Z) = (X + Y)(!X + Z)
//     // rewrite!("th8"; "(* (+ ?x ?y) (* (+ (! ?x) ?z) (+ ?y ?z)))" => "(* (+ ?x ?y) (+ (! ?x) ?z))")


//     ]
    
// }
