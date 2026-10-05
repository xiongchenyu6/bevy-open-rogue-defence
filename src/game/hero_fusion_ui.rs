//! Readable inherited combat identities, attached to the scrollable hero sheet.
use crate::game::{
    GameLanguage, HeroFusion, HeroKind, PlayerBuild,
    content::HEROES,
    localization::{hero_name, hero_ultimate_name},
};
use bevy::prelude::*;
fn weapon(language: GameLanguage, kind: HeroKind) -> &'static str {
    let names = [
        ("Lightning bolt", "雷霆弹"),
        ("Flaming crossbow", "烈焰弩"),
        ("Stone sword", "岩刃"),
        ("Spirit orb", "灵体法球"),
        ("Tidal daggers", "潮汐双刃"),
        ("Arcane star", "奥术星弹"),
        ("Frost shard", "寒冰棱晶"),
        ("Blood axe", "血斧"),
        ("Rift daggers", "裂隙双刃"),
        ("Mechanical crossbow", "机巧弩"),
        ("Solar sword", "日耀剑"),
        ("Void axe", "虚空斧"),
    ];
    let (en, zh) = names[kind.index()];
    language.pick(en, zh)
}
pub(super) fn spawn(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    language: GameLanguage,
    fusion: &HeroFusion,
    build: &PlayerBuild,
    level: u32,
    cleave_bonus: usize,
    compact: bool,
) {
    if fusion.count() == 0 {
        return;
    }
    parent.spawn((Name::new("Hero inherited abilities"),Node {flex_direction:FlexDirection::Column,row_gap:px(7),flex_shrink:0.0,padding:UiRect::top(px(8)),..default()})).with_children(|section| {
        section.spawn((Text::new(format!("{} · {} / {}",language.pick("INHERITED ARSENAL · SKILLS","吞噬传承 · 技能槽"),build.skills.iter().filter(|rank|**rank>0).count(),fusion.skill_capacity())),TextFont {font_size:FontSize::Px(if compact {13.0}else{16.0}),..default()},TextColor(Color::srgb(0.95,0.75,0.4))));
        section.spawn((Text::new(language.pick("Every fourth basic attack adds one inherited weapon. Passives remain active; ultimates rotate automatically.","每四次普攻追加一种继承武器；被动持续生效，终结技自动轮流释放。")),TextFont {font_size:FontSize::Px(11.0),..default()},TextColor(Color::srgb(0.66,0.74,0.76))));
        for kind in HeroKind::ALL.into_iter().filter(|kind|fusion.has(*kind)) {
            section.spawn((Name::new(format!("Inherited hero {}",HEROES[kind.index()].id)),Node {min_height:px(62),column_gap:px(10),align_items:AlignItems::Center,flex_shrink:0.0,padding:UiRect::all(px(6)),..default()},BackgroundColor(Color::srgb(0.045,0.075,0.085)))).with_children(|row| {
                row.spawn((ImageNode::new(assets.load(format!("ui/portraits/hero_{}.png",HEROES[kind.index()].id))),Node {width:px(48),height:px(48),flex_shrink:0.0,..default()},Pickable::IGNORE));
                row.spawn(Node {flex_direction:FlexDirection::Column,row_gap:px(3),min_width:px(0),..default()}).with_children(|detail| {
                    detail.spawn((Name::new(format!("Inherited basic ability {}",HEROES[kind.index()].id)),Text::new(crate::game::basic_attack_details::summary(language,kind,level,build,cleave_bonus)),TextFont {font_size:FontSize::Px(11.0),..default()},TextColor(Color::srgb(0.95,0.78,0.42))));
                    let passive_en=["Echo Formula","Hunt Rhythm","Gatekeeper","Spirit Pack","Tidal Boundary","Lucky Chips","Ice Prophecy","Blood Sustain","Phase Step","Living Repairs","Dawn Oath","Void Echo"];
                    detail.spawn((Text::new(format!("{} · {}",hero_name(language,kind),weapon(language,kind))),TextFont {font_size:FontSize::Px(if compact {12.0}else{14.0}),..default()},TextColor(Color::srgb(0.97,0.92,0.8))));
                    detail.spawn((Text::new(format!("{} · {}",language.pick(passive_en[kind.index()],HEROES[kind.index()].passive.name),hero_ultimate_name(language,kind))),TextFont {font_size:FontSize::Px(11.0),..default()},TextColor(Color::srgb(0.72,0.82,0.84))));
                });
            });
        }
    });
}
