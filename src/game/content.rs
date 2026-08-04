//! Static, engine-agnostic content definitions for the first playable ruleset.
//!
//! IDs are stable ASCII strings so this table can later move to RON without
//! changing save data. Display names and fiction are original to this project.

pub type ContentId = &'static str;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rarity {
    R,
    Sr,
    Ssr,
    Ur,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    Attack,
    Spell,
    Summon,
    Storm,
    Flame,
    Frost,
    Tide,
    Physical,
    Arcane,
    Qi,
    Guard,
    Fortune,
    Control,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attribute {
    Strength,
    Agility,
    Intelligence,
}

#[derive(Debug, Clone, Copy)]
pub struct Stats {
    pub strength: f32,
    pub agility: f32,
    pub intelligence: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct PassiveDef {
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct UltimateDef {
    pub name: &'static str,
    pub cooldown_secs: f32,
    pub tags: &'static [Tag],
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct HeroDef {
    pub id: ContentId,
    pub name: &'static str,
    pub rarity: Rarity,
    pub primary: Attribute,
    pub tags: &'static [Tag],
    pub base_stats: Stats,
    pub growth: Stats,
    pub base_health: f32,
    pub base_energy: f32,
    pub starting_skill: ContentId,
    pub passive: PassiveDef,
    pub ultimate: UltimateDef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillEffectKind {
    Projectile,
    Nova,
    GroundZone,
    AttackProc,
    Orbit,
    Summon,
    Mark,
    Echo,
    Revive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillTrigger {
    Cooldown,
    Passive,
    EveryNthAttack(u8),
    EveryNthSpell(u8),
}

#[derive(Debug, Clone, Copy)]
pub struct ScaledValue {
    pub base: f32,
    pub per_rank: f32,
    pub attack_ratio: f32,
    pub spell_ratio: f32,
    pub summon_ratio: f32,
}

impl ScaledValue {
    pub const ZERO: Self = Self {
        base: 0.0,
        per_rank: 0.0,
        attack_ratio: 0.0,
        spell_ratio: 0.0,
        summon_ratio: 0.0,
    };
}

#[derive(Debug, Clone, Copy)]
pub struct SkillEffect {
    pub kind: SkillEffectKind,
    pub power: ScaledValue,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct SkillDef {
    pub id: ContentId,
    pub name: &'static str,
    pub rarity: Rarity,
    pub tags: &'static [Tag],
    pub trigger: SkillTrigger,
    pub cooldown_secs: f32,
    pub max_rank: u8,
    pub effect: SkillEffect,
}

#[derive(Debug, Clone, Copy)]
pub struct BondTier {
    pub threshold: u8,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct BondDef {
    pub id: ContentId,
    pub name: &'static str,
    pub rarity: Rarity,
    pub favored_tags: &'static [Tag],
    pub tiers: [BondTier; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyBehavior {
    MarchToGate,
    Swarm,
    FrontalGuard,
    HuntHero,
    RushGate,
    HasteAura,
    Unstoppable,
    BurrowAmbush,
    HealAura,
    SplitOnDeath,
    SilenceAura,
    SiegeGate,
}

#[derive(Debug, Clone, Copy)]
pub struct EnemyDef {
    pub id: ContentId,
    pub name: &'static str,
    pub hp: f32,
    pub damage: f32,
    pub armor: f32,
    pub speed: f32,
    pub gold_bounty: u16,
    pub kill_credit: u16,
    pub xp: u16,
    pub breach_damage: u16,
    pub behavior: EnemyBehavior,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct BossModifiers {
    pub max_hp_multiplier: f32,
    pub move_speed_multiplier: f32,
    pub attack_speed_multiplier: f32,
    pub damage_reduction: f32,
    pub elemental_resistance: f32,
    pub flame_resistance: f32,
    pub frost_damage_taken_multiplier: f32,
}

impl BossModifiers {
    pub const NEUTRAL: Self = Self {
        max_hp_multiplier: 1.0,
        move_speed_multiplier: 1.0,
        attack_speed_multiplier: 1.0,
        damage_reduction: 0.0,
        elemental_resistance: 0.0,
        flame_resistance: 0.0,
        frost_damage_taken_multiplier: 1.0,
    };
}

#[derive(Debug, Clone, Copy)]
pub struct BossDef {
    pub id: ContentId,
    pub name: &'static str,
    pub spawn_seconds: u16,
    pub hp: f32,
    pub gate_damage: u16,
    pub modifiers: BossModifiers,
    pub mechanics: &'static [&'static str],
}

pub const HEROES: [HeroDef; 6] = [
    HeroDef {
        id: "storm_savant",
        name: "玄霆术师",
        rarity: Rarity::R,
        primary: Attribute::Intelligence,
        tags: &[Tag::Spell, Tag::Storm, Tag::Arcane],
        base_stats: Stats {
            strength: 20.0,
            agility: 18.0,
            intelligence: 28.0,
        },
        growth: Stats {
            strength: 2.0,
            agility: 2.0,
            intelligence: 3.4,
        },
        base_health: 520.0,
        base_energy: 280.0,
        starting_skill: "arc_chain",
        passive: PassiveDef {
            name: "余响算式",
            description: "每累积六枚法术投射物，追加一发40%攻击伤害的雷霆回响。",
        },
        ultimate: UltimateDef {
            name: "雷域",
            cooldown_secs: 50.0,
            tags: &[Tag::Spell, Tag::Storm],
            description: "展开6秒雷域，每0.6秒轰击一组敌人。",
        },
    },
    HeroDef {
        id: "ember_ranger",
        name: "烬弦猎手",
        rarity: Rarity::R,
        primary: Attribute::Agility,
        tags: &[Tag::Attack, Tag::Flame, Tag::Physical],
        base_stats: Stats {
            strength: 18.0,
            agility: 30.0,
            intelligence: 18.0,
        },
        growth: Stats {
            strength: 2.0,
            agility: 3.5,
            intelligence: 1.8,
        },
        base_health: 500.0,
        base_energy: 210.0,
        starting_skill: "ember_arrow",
        passive: PassiveDef {
            name: "追猎节拍",
            description: "连续攻击叠加攻速；移动时层数缓慢衰减。",
        },
        ultimate: UltimateDef {
            name: "赤羽齐射",
            cooldown_secs: 45.0,
            tags: &[Tag::Attack, Tag::Flame],
            description: "连续五轮向敌群自动瞄准的扇形箭雨。",
        },
    },
    HeroDef {
        id: "stone_warden",
        name: "山铠守卫",
        rarity: Rarity::R,
        primary: Attribute::Strength,
        tags: &[Tag::Guard, Tag::Physical],
        base_stats: Stats {
            strength: 32.0,
            agility: 16.0,
            intelligence: 16.0,
        },
        growth: Stats {
            strength: 3.8,
            agility: 1.7,
            intelligence: 1.6,
        },
        base_health: 680.0,
        base_energy: 190.0,
        starting_skill: "ground_pulse",
        passive: PassiveDef {
            name: "守门山势",
            description: "英雄位于城门8米内时，返还城门所受伤害的15%。",
        },
        ultimate: UltimateDef {
            name: "山岳锚定",
            cooldown_secs: 50.0,
            tags: &[Tag::Guard, Tag::Control],
            description: "附近敌人减速72%持续6秒，并立即修复90城门耐久。",
        },
    },
    HeroDef {
        id: "spirit_keeper",
        name: "灵契牧者",
        rarity: Rarity::Sr,
        primary: Attribute::Intelligence,
        tags: &[Tag::Summon, Tag::Guard],
        base_stats: Stats {
            strength: 22.0,
            agility: 18.0,
            intelligence: 29.0,
        },
        growth: Stats {
            strength: 2.3,
            agility: 1.8,
            intelligence: 3.2,
        },
        base_health: 550.0,
        base_energy: 300.0,
        starting_skill: "bone_sentry",
        passive: PassiveDef {
            name: "契群",
            description: "在共享召唤上限内维持一只永久灵体，以110%攻击伤害协战。",
        },
        ultimate: UltimateDef {
            name: "万灵行军",
            cooldown_secs: 55.0,
            tags: &[Tag::Summon, Tag::Physical],
            description: "在共享召唤槽内召来最多六只灵兽协战12秒。",
        },
    },
    HeroDef {
        id: "tide_walker",
        name: "潮界行者",
        rarity: Rarity::Sr,
        primary: Attribute::Intelligence,
        tags: &[Tag::Spell, Tag::Tide, Tag::Frost],
        base_stats: Stats {
            strength: 24.0,
            agility: 22.0,
            intelligence: 25.0,
        },
        growth: Stats {
            strength: 2.5,
            agility: 2.4,
            intelligence: 2.8,
        },
        base_health: 590.0,
        base_energy: 270.0,
        starting_skill: "tidal_fan",
        passive: PassiveDef {
            name: "潮霜交界",
            description: "每3.2秒在最近敌人间交替施加潮湿与冻结，并触发一次潮霜伤害。",
        },
        ultimate: UltimateDef {
            name: "静海",
            cooldown_secs: 52.0,
            tags: &[Tag::Tide, Tag::Frost, Tag::Control],
            description: "冻结普通敌人3秒；Boss减速60%持续5秒。",
        },
    },
    HeroDef {
        id: "star_wagerer",
        name: "星筹浪客",
        rarity: Rarity::Sr,
        primary: Attribute::Agility,
        tags: &[Tag::Fortune, Tag::Arcane],
        base_stats: Stats {
            strength: 20.0,
            agility: 28.0,
            intelligence: 22.0,
        },
        growth: Stats {
            strength: 2.0,
            agility: 3.1,
            intelligence: 2.4,
        },
        base_health: 520.0,
        base_energy: 240.0,
        starting_skill: "arcane_star",
        passive: PassiveDef {
            name: "有利筹码",
            description: "每累计12个有效击杀获得18金币和1木材。",
        },
        ultimate: UltimateDef {
            name: "满载命运",
            cooldown_secs: 48.0,
            tags: &[Tag::Fortune, Tag::Arcane],
            description: "城门低于65%时治疗；敌人至少8名时攻击，否则获得资源。",
        },
    },
];

pub const SKILLS: [SkillDef; 30] = [
    SkillDef {
        id: "arc_chain",
        name: "弧光链",
        rarity: Rarity::R,
        tags: &[Tag::Spell, Tag::Storm, Tag::Arcane],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 3.6,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Projectile,
            power: ScaledValue {
                base: 45.0,
                per_rank: 28.0,
                attack_ratio: 0.0,
                spell_ratio: 0.55,
                summon_ratio: 0.0,
            },
            description: "弹射3次；技能等级每提高2级，额外弹射1次。",
        },
    },
    SkillDef {
        id: "ember_arrow",
        name: "烬矢",
        rarity: Rarity::R,
        tags: &[Tag::Attack, Tag::Flame, Tag::Physical],
        trigger: SkillTrigger::EveryNthAttack(3),
        cooldown_secs: 0.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::AttackProc,
            power: ScaledValue {
                base: 25.0,
                per_rank: 18.0,
                attack_ratio: 0.55,
                spell_ratio: 0.0,
                summon_ratio: 0.0,
            },
            description: "附加伤害并在3秒内造成等量燃烧；满级改为每两次攻击触发。",
        },
    },
    SkillDef {
        id: "frost_lance",
        name: "霜棱",
        rarity: Rarity::R,
        tags: &[Tag::Spell, Tag::Frost, Tag::Control],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 4.4,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Projectile,
            power: ScaledValue {
                base: 55.0,
                per_rank: 34.0,
                attack_ratio: 0.0,
                spell_ratio: 0.65,
                summon_ratio: 0.0,
            },
            description: "穿透两个目标并减速30%；3级双发，5级冻结。",
        },
    },
    SkillDef {
        id: "tidal_fan",
        name: "潮刃扇",
        rarity: Rarity::R,
        tags: &[Tag::Spell, Tag::Tide],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 5.2,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Projectile,
            power: ScaledValue {
                base: 24.0,
                per_rank: 14.0,
                attack_ratio: 0.0,
                spell_ratio: 0.28,
                summon_ratio: 0.0,
            },
            description: "发射五枚水刃；单个目标最多被同一轮命中两次。",
        },
    },
    SkillDef {
        id: "bone_sentry",
        name: "骨卫",
        rarity: Rarity::R,
        tags: &[Tag::Summon, Tag::Guard],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 10.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Summon,
            power: ScaledValue {
                base: 18.0,
                per_rank: 11.0,
                attack_ratio: 0.0,
                spell_ratio: 0.0,
                summon_ratio: 0.25,
            },
            description: "召唤守门骨卫；等级提高生命、攻击与同时存在上限。",
        },
    },
    SkillDef {
        id: "spirit_hound",
        name: "灵猎群",
        rarity: Rarity::R,
        tags: &[Tag::Summon, Tag::Physical],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 14.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Summon,
            power: ScaledValue {
                base: 32.0,
                per_rank: 14.0,
                attack_ratio: 0.0,
                spell_ratio: 0.0,
                summon_ratio: 0.35,
            },
            description: "召唤2至4只灵兽追猎敌群，持续10秒。",
        },
    },
    SkillDef {
        id: "orbit_blade",
        name: "回旋刃",
        rarity: Rarity::R,
        tags: &[Tag::Attack, Tag::Physical, Tag::Qi],
        trigger: SkillTrigger::Passive,
        cooldown_secs: 0.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Orbit,
            power: ScaledValue {
                base: 28.0,
                per_rank: 15.0,
                attack_ratio: 0.35,
                spell_ratio: 0.0,
                summon_ratio: 0.0,
            },
            description: "维持2至4枚环刃；同一目标每秒最多受击一次。",
        },
    },
    SkillDef {
        id: "piercing_volley",
        name: "贯星连射",
        rarity: Rarity::R,
        tags: &[Tag::Attack, Tag::Physical],
        trigger: SkillTrigger::EveryNthAttack(4),
        cooldown_secs: 0.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::AttackProc,
            power: ScaledValue {
                base: 0.0,
                per_rank: 0.0,
                attack_ratio: 0.6,
                spell_ratio: 0.0,
                summon_ratio: 0.0,
            },
            description: "发射3至5支贯穿箭，每支造成60%攻击伤害。",
        },
    },
    SkillDef {
        id: "arcane_star",
        name: "秘能飞星",
        rarity: Rarity::R,
        tags: &[Tag::Spell, Tag::Arcane],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 4.8,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Nova,
            power: ScaledValue {
                base: 60.0,
                per_rank: 35.0,
                attack_ratio: 0.0,
                spell_ratio: 0.7,
                summon_ratio: 0.0,
            },
            description: "飞星命中后在小范围爆裂。",
        },
    },
    SkillDef {
        id: "ground_pulse",
        name: "裂地劲",
        rarity: Rarity::R,
        tags: &[Tag::Spell, Tag::Physical, Tag::Guard],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 6.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Nova,
            power: ScaledValue {
                base: 65.0,
                per_rank: 40.0,
                attack_ratio: 0.65,
                spell_ratio: 0.0,
                summon_ratio: 0.0,
            },
            description: "向城门前方震出锥形冲击并眩晕0.6秒。",
        },
    },
    SkillDef {
        id: "static_seal",
        name: "感电印",
        rarity: Rarity::R,
        tags: &[Tag::Spell, Tag::Storm],
        trigger: SkillTrigger::Passive,
        cooldown_secs: 0.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Mark,
            power: ScaledValue {
                base: 70.0,
                per_rank: 40.0,
                attack_ratio: 0.0,
                spell_ratio: 0.8,
                summon_ratio: 0.0,
            },
            description: "法术附加感电；四层时爆炸并清除印记。",
        },
    },
    SkillDef {
        id: "cinder_aura",
        name: "焚身环",
        rarity: Rarity::R,
        tags: &[Tag::Spell, Tag::Flame],
        trigger: SkillTrigger::Passive,
        cooldown_secs: 0.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::GroundZone,
            power: ScaledValue {
                base: 12.0,
                per_rank: 8.0,
                attack_ratio: 0.0,
                spell_ratio: 0.15,
                summon_ratio: 0.0,
            },
            description: "每0.5秒灼烧英雄身边的敌人。",
        },
    },
    SkillDef {
        id: "storm_matrix",
        name: "雷阵",
        rarity: Rarity::Sr,
        tags: &[Tag::Spell, Tag::Storm],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 9.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::GroundZone,
            power: ScaledValue {
                base: 25.0,
                per_rank: 14.0,
                attack_ratio: 0.0,
                spell_ratio: 0.22,
                summon_ratio: 0.0,
            },
            description: "雷区持续4秒，每0.6秒脉冲一次。",
        },
    },
    SkillDef {
        id: "meteor_seed",
        name: "陨星种",
        rarity: Rarity::Sr,
        tags: &[Tag::Spell, Tag::Flame],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 10.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Nova,
            power: ScaledValue {
                base: 180.0,
                per_rank: 100.0,
                attack_ratio: 0.0,
                spell_ratio: 1.6,
                summon_ratio: 0.0,
            },
            description: "显示0.8秒落点预警后造成大范围火焰伤害。",
        },
    },
    SkillDef {
        id: "glacier_core",
        name: "冰核",
        rarity: Rarity::Sr,
        tags: &[Tag::Spell, Tag::Frost, Tag::Arcane],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 8.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::GroundZone,
            power: ScaledValue {
                base: 32.0,
                per_rank: 18.0,
                attack_ratio: 0.0,
                spell_ratio: 0.3,
                summon_ratio: 0.0,
            },
            description: "冰核持续5秒脉冲；同一目标累计三次后冻结。",
        },
    },
    SkillDef {
        id: "vortex_column",
        name: "涡柱",
        rarity: Rarity::Sr,
        tags: &[Tag::Spell, Tag::Tide, Tag::Storm, Tag::Control],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 11.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::GroundZone,
            power: ScaledValue {
                base: 120.0,
                per_rank: 70.0,
                attack_ratio: 0.0,
                spell_ratio: 1.2,
                summon_ratio: 0.0,
            },
            description: "聚拢普通敌人并在4秒内持续造成伤害。",
        },
    },
    SkillDef {
        id: "tide_elemental",
        name: "潮灵",
        rarity: Rarity::Sr,
        tags: &[Tag::Summon, Tag::Tide],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 12.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Summon,
            power: ScaledValue {
                base: 30.0,
                per_rank: 16.0,
                attack_ratio: 0.0,
                spell_ratio: 0.0,
                summon_ratio: 0.45,
            },
            description: "召唤远程潮灵；攻击施加潮湿，死亡时修复城门。",
        },
    },
    SkillDef {
        id: "bolt_turret",
        name: "雷弩塔",
        rarity: Rarity::Sr,
        tags: &[Tag::Summon, Tag::Storm, Tag::Guard],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 12.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Summon,
            power: ScaledValue {
                base: 38.0,
                per_rank: 20.0,
                attack_ratio: 0.0,
                spell_ratio: 0.0,
                summon_ratio: 0.4,
            },
            description: "部署16秒固定炮塔，优先射击冲城单位。",
        },
    },
    SkillDef {
        id: "execution_brand",
        name: "处决印",
        rarity: Rarity::Sr,
        tags: &[Tag::Attack, Tag::Physical],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 8.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Mark,
            power: ScaledValue::ZERO,
            description: "标记最高生命敌人；杂兵12%、精英与Boss 4%生命以下处决。",
        },
    },
    SkillDef {
        id: "flying_swords",
        name: "御剑阵",
        rarity: Rarity::Sr,
        tags: &[Tag::Spell, Tag::Qi, Tag::Physical],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 2.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Projectile,
            power: ScaledValue {
                base: 35.0,
                per_rank: 20.0,
                attack_ratio: 0.0,
                spell_ratio: 0.35,
                summon_ratio: 0.0,
            },
            description: "放出1至3柄自动寻敌的飞剑。",
        },
    },
    SkillDef {
        id: "prism_echo",
        name: "棱镜回响",
        rarity: Rarity::Sr,
        tags: &[Tag::Spell, Tag::Arcane],
        trigger: SkillTrigger::EveryNthSpell(5),
        cooldown_secs: 0.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Echo,
            power: ScaledValue {
                base: 0.35,
                per_rank: 0.1,
                attack_ratio: 0.0,
                spell_ratio: 0.0,
                summon_ratio: 0.0,
            },
            description: "复制上一个非回响技能，效果随等级从35%提升。",
        },
    },
    SkillDef {
        id: "oath_banner",
        name: "守誓旗",
        rarity: Rarity::Sr,
        tags: &[Tag::Summon, Tag::Guard],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 16.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Summon,
            power: ScaledValue::ZERO,
            description: "旗帜存在10秒，提供减伤与攻速，并每秒修复城门1点。",
        },
    },
    SkillDef {
        id: "transmute_spark",
        name: "炼金火花",
        rarity: Rarity::Sr,
        tags: &[Tag::Spell, Tag::Fortune, Tag::Flame],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 7.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Nova,
            power: ScaledValue {
                base: 90.0,
                per_rank: 50.0,
                attack_ratio: 0.0,
                spell_ratio: 0.8,
                summon_ratio: 0.0,
            },
            description: "爆炸击杀目标时有4%至12%概率额外掉落金币。",
        },
    },
    SkillDef {
        id: "blood_wheel",
        name: "血轮",
        rarity: Rarity::Sr,
        tags: &[Tag::Attack, Tag::Physical, Tag::Flame],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 8.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Projectile,
            power: ScaledValue {
                base: 0.0,
                per_rank: 0.0,
                attack_ratio: 0.9,
                spell_ratio: 0.0,
                summon_ratio: 0.0,
            },
            description: "巨轮往返穿过战线；回程造成等量燃烧并可重复命中。",
        },
    },
    SkillDef {
        id: "sky_judgment",
        name: "天穹裁决",
        rarity: Rarity::Ur,
        tags: &[Tag::Spell, Tag::Storm],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 14.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Nova,
            power: ScaledValue {
                base: 130.0,
                per_rank: 65.0,
                attack_ratio: 0.0,
                spell_ratio: 1.1,
                summon_ratio: 0.0,
            },
            description: "锁定6至10个目标，各降下一次裁决雷击。",
        },
    },
    SkillDef {
        id: "ember_rebirth",
        name: "余烬返生",
        rarity: Rarity::Ur,
        tags: &[Tag::Flame, Tag::Guard],
        trigger: SkillTrigger::Passive,
        cooldown_secs: 90.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Revive,
            power: ScaledValue {
                base: 240.0,
                per_rank: 120.0,
                attack_ratio: 0.0,
                spell_ratio: 1.8,
                summon_ratio: 0.0,
            },
            description: "免除一次致命伤，恢复35%生命并产生范围爆燃。",
        },
    },
    SkillDef {
        id: "frost_wyrm",
        name: "霜翼",
        rarity: Rarity::Ur,
        tags: &[Tag::Summon, Tag::Frost, Tag::Tide],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 18.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Summon,
            power: ScaledValue {
                base: 80.0,
                per_rank: 42.0,
                attack_ratio: 0.0,
                spell_ratio: 0.0,
                summon_ratio: 0.8,
            },
            description: "召唤原创霜翼兽20秒；吐息穿透并累积冻结。",
        },
    },
    SkillDef {
        id: "ancestor_colossus",
        name: "先祖巨像",
        rarity: Rarity::Ur,
        tags: &[Tag::Summon, Tag::Guard, Tag::Physical],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 22.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Summon,
            power: ScaledValue {
                base: 95.0,
                per_rank: 50.0,
                attack_ratio: 0.0,
                spell_ratio: 0.0,
                summon_ratio: 1.0,
            },
            description: "大型守门召唤物持续牵引非Boss敌人并每3秒震地，持续18秒。",
        },
    },
    SkillDef {
        id: "triune_engine",
        name: "三仪轮",
        rarity: Rarity::Ur,
        tags: &[Tag::Attack, Tag::Spell, Tag::Summon, Tag::Arcane],
        trigger: SkillTrigger::Passive,
        cooldown_secs: 0.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Orbit,
            power: ScaledValue::ZERO,
            description: "每4秒轮换强化攻击、法术或召唤；满级时三种强化常驻。",
        },
    },
    SkillDef {
        id: "fate_cube",
        name: "命运立方",
        rarity: Rarity::Ur,
        tags: &[Tag::Fortune, Tag::Arcane],
        trigger: SkillTrigger::Cooldown,
        cooldown_secs: 12.0,
        max_rank: 5,
        effect: SkillEffect {
            kind: SkillEffectKind::Echo,
            power: ScaledValue::ZERO,
            description: "随机触发雷暴、火雨、治疗、金币或召唤；等级提高高阶结果权重。",
        },
    },
];

pub const BONDS: [BondDef; 10] = [
    BondDef {
        id: "storm_conduit",
        name: "天雷导体",
        rarity: Rarity::R,
        favored_tags: &[Tag::Storm, Tag::Spell],
        tiers: [
            BondTier {
                threshold: 2,
                description: "雷电伤害+15%，风暴命中附加感电。",
            },
            BondTier {
                threshold: 4,
                description: "风暴冷却-12%，弹射或脉冲次数+1。",
            },
            BondTier {
                threshold: 6,
                description: "每8次风暴命中追加一次高倍率落雷。",
            },
        ],
    },
    BondDef {
        id: "ember_oath",
        name: "炽痕誓约",
        rarity: Rarity::R,
        favored_tags: &[Tag::Flame, Tag::Attack],
        tiers: [
            BondTier {
                threshold: 2,
                description: "燃烧上限提高到5层，每层提高目标所受火焰伤害。",
            },
            BondTier {
                threshold: 4,
                description: "燃烧可暴击，并在目标死亡时向邻近敌人扩散。",
            },
            BondTier {
                threshold: 6,
                description: "满层燃烧每2秒触发一次爆燃。",
            },
        ],
    },
    BondDef {
        id: "spirit_pact",
        name: "万灵契约",
        rarity: Rarity::R,
        favored_tags: &[Tag::Summon, Tag::Guard],
        tiers: [
            BondTier {
                threshold: 2,
                description: "召唤上限+2，并提高属性继承。",
            },
            BondTier {
                threshold: 4,
                description: "召唤攻击标记目标；召唤死亡时爆炸。",
            },
            BondTier {
                threshold: 6,
                description: "每12秒生成一个持续8秒的召唤镜像。",
            },
        ],
    },
    BondDef {
        id: "element_cycle",
        name: "源素轮转",
        rarity: Rarity::Sr,
        favored_tags: &[Tag::Storm, Tag::Flame, Tag::Frost, Tag::Tide],
        tiers: [
            BondTier {
                threshold: 2,
                description: "开启潮湿、风暴、燃烧、冰霜之间的四种反应。",
            },
            BondTier {
                threshold: 4,
                description: "元素反应伤害+60%，状态持续时间+35%。",
            },
            BondTier {
                threshold: 6,
                description: "单目标反应冷却降至1秒，并产生一次回响。",
            },
        ],
    },
    BondDef {
        id: "arcane_loop",
        name: "秘仪回路",
        rarity: Rarity::R,
        favored_tags: &[Tag::Spell, Tag::Arcane],
        tiers: [
            BondTier {
                threshold: 2,
                description: "法术冷却-10%。",
            },
            BondTier {
                threshold: 4,
                description: "每第5次法术以50%效果回响。",
            },
            BondTier {
                threshold: 6,
                description: "回响提高至100%，但不能递归触发。",
            },
        ],
    },
    BondDef {
        id: "triune_union",
        name: "三元合流",
        rarity: Rarity::Sr,
        favored_tags: &[Tag::Attack, Tag::Spell, Tag::Summon],
        tiers: [
            BondTier {
                threshold: 2,
                description: "三类命中分别提供8%对应增伤。",
            },
            BondTier {
                threshold: 4,
                description: "4秒内集齐三类命中时触发三元爆破。",
            },
            BondTier {
                threshold: 6,
                description: "三类增益常驻，爆破的25%转为真实伤害。",
            },
        ],
    },
    BondDef {
        id: "qi_ascension",
        name: "炼气登临",
        rarity: Rarity::Sr,
        favored_tags: &[Tag::Qi, Tag::Spell],
        tiers: [
            BondTier {
                threshold: 2,
                description: "每10次施法放出一柄气剑。",
            },
            BondTier {
                threshold: 4,
                description: "4级及以上技能伤害+25%。",
            },
            BondTier {
                threshold: 6,
                description: "四个技能满级后登临：全冷却-20%，每6秒剑雨。",
            },
        ],
    },
    BondDef {
        id: "blood_array",
        name: "血战杀阵",
        rarity: Rarity::R,
        favored_tags: &[Tag::Attack, Tag::Physical],
        tiers: [
            BondTier {
                threshold: 2,
                description: "暴击率+10%，击杀恢复1%生命。",
            },
            BondTier {
                threshold: 4,
                description: "处决低生命杂兵，目标死亡时爆炸。",
            },
            BondTier {
                threshold: 6,
                description: "每20次击杀进入10秒血战，攻速与施法速度+35%。",
            },
        ],
    },
    BondDef {
        id: "raider_fortune",
        name: "荒市豪赌",
        rarity: Rarity::Sr,
        favored_tags: &[Tag::Fortune],
        tiers: [
            BondTier {
                threshold: 2,
                description: "金币收益+20%，黑市杀敌价格-10%。",
            },
            BondTier {
                threshold: 4,
                description: "每轮黑市首次刷新免费，稀有率+15%。",
            },
            BondTier {
                threshold: 6,
                description: "幸运结果投两次取优，精英必掉消耗品。",
            },
        ],
    },
    BondDef {
        id: "guardian_oath",
        name: "守誓壁垒",
        rarity: Rarity::R,
        favored_tags: &[Tag::Guard],
        tiers: [
            BondTier {
                threshold: 2,
                description: "城门耐久+250，城门附近英雄减伤10%。",
            },
            BondTier {
                threshold: 4,
                description: "每12秒拦截一次突破。",
            },
            BondTier {
                threshold: 6,
                description: "城门高耐久时增伤；低耐久时触发一次6秒无敌。",
            },
        ],
    },
];

pub const ENEMIES: [EnemyDef; 12] = [
    EnemyDef {
        id: "rift_drone",
        name: "荒潮工兵",
        hp: 55.0,
        damage: 8.0,
        armor: 0.0,
        speed: 2.1,
        gold_bounty: 2,
        kill_credit: 1,
        xp: 4,
        breach_damage: 8,
        behavior: EnemyBehavior::MarchToGate,
        description: "从右侧出生带向左侧城门稳定推进的基础近战单位。",
    },
    EnemyDef {
        id: "gnawling",
        name: "噬群幼兽",
        hp: 28.0,
        damage: 5.0,
        armor: 0.0,
        speed: 3.4,
        gold_bounty: 1,
        kill_credit: 1,
        xp: 2,
        breach_damage: 4,
        behavior: EnemyBehavior::Swarm,
        description: "高速成群推进，以数量压缩英雄走位空间。",
    },
    EnemyDef {
        id: "shell_guard",
        name: "甲壳卫",
        hp: 150.0,
        damage: 12.0,
        armor: 8.0,
        speed: 1.5,
        gold_bounty: 4,
        kill_credit: 1,
        xp: 7,
        breach_damage: 14,
        behavior: EnemyBehavior::FrontalGuard,
        description: "正面受到的伤害降低40%，鼓励侧移或范围攻击。",
    },
    EnemyDef {
        id: "acid_archer",
        name: "酸羽射手",
        hp: 70.0,
        damage: 9.0,
        armor: 1.0,
        speed: 1.8,
        gold_bounty: 3,
        kill_credit: 1,
        xp: 5,
        breach_damage: 9,
        behavior: EnemyBehavior::HuntHero,
        description: "进入射程后停步并优先攻击英雄。",
    },
    EnemyDef {
        id: "core_rusher",
        name: "冲城兽",
        hp: 80.0,
        damage: 0.0,
        armor: 0.0,
        speed: 3.0,
        gold_bounty: 3,
        kill_credit: 1,
        xp: 4,
        breach_damage: 20,
        behavior: EnemyBehavior::RushGate,
        description: "忽略英雄，沿横向战线直接冲击左侧城门。",
    },
    EnemyDef {
        id: "war_chanter",
        name: "鼓噪祭者",
        hp: 95.0,
        damage: 7.0,
        armor: 2.0,
        speed: 1.7,
        gold_bounty: 4,
        kill_credit: 1,
        xp: 7,
        breach_damage: 10,
        behavior: EnemyBehavior::HasteAura,
        description: "使附近敌人的移动速度与攻击速度提高20%。",
    },
    EnemyDef {
        id: "stone_brute",
        name: "岩背巨像",
        hp: 320.0,
        damage: 26.0,
        armor: 12.0,
        speed: 1.0,
        gold_bounty: 9,
        kill_credit: 5,
        xp: 14,
        breach_damage: 24,
        behavior: EnemyBehavior::Unstoppable,
        description: "缓慢而坚固，免疫普通击退。",
    },
    EnemyDef {
        id: "burrow_stalker",
        name: "遁地刺客",
        hp: 90.0,
        damage: 16.0,
        armor: 2.0,
        speed: 2.5,
        gold_bounty: 4,
        kill_credit: 1,
        xp: 7,
        breach_damage: 12,
        behavior: EnemyBehavior::BurrowAmbush,
        description: "潜地越过部分战线，在城门附近现身并短暂眩晕英雄。",
    },
    EnemyDef {
        id: "blood_mender",
        name: "血巢医师",
        hp: 110.0,
        damage: 5.0,
        armor: 2.0,
        speed: 1.6,
        gold_bounty: 5,
        kill_credit: 1,
        xp: 8,
        breach_damage: 10,
        behavior: EnemyBehavior::HealAura,
        description: "每秒治疗附近敌人2%最大生命。",
    },
    EnemyDef {
        id: "split_spore",
        name: "裂生孢体",
        hp: 140.0,
        damage: 10.0,
        armor: 1.0,
        speed: 1.7,
        gold_bounty: 4,
        kill_credit: 1,
        xp: 7,
        breach_damage: 12,
        behavior: EnemyBehavior::SplitOnDeath,
        description: "死亡分裂为三只幼体；幼体不再提供资源。",
    },
    EnemyDef {
        id: "null_eye",
        name: "沉默眼",
        hp: 120.0,
        damage: 9.0,
        armor: 3.0,
        speed: 1.5,
        gold_bounty: 6,
        kill_credit: 1,
        xp: 9,
        breach_damage: 14,
        behavior: EnemyBehavior::SilenceAura,
        description: "提高邻近敌人的魔抗，并降低附近英雄的冷却恢复。",
    },
    EnemyDef {
        id: "siege_lobber",
        name: "攻城投手",
        hp: 200.0,
        damage: 18.0,
        armor: 6.0,
        speed: 1.2,
        gold_bounty: 8,
        kill_credit: 5,
        xp: 12,
        breach_damage: 28,
        behavior: EnemyBehavior::SiegeGate,
        description: "远距离蓄力轰击城门，攻击前显示可躲避的落点。",
    },
];

pub const BOSSES: [BossDef; 5] = [
    BossDef {
        id: "cinder_colossus",
        name: "炽烬巨像",
        spawn_seconds: 5 * 60,
        hp: 8_000.0,
        gate_damage: 120,
        modifiers: BossModifiers {
            flame_resistance: 0.70,
            frost_damage_taken_multiplier: 1.35,
            ..BossModifiers::NEUTRAL
        },
        mechanics: &[
            "火焰抗性极高、额外承受冰霜伤害，首个检查点要求元素反制。",
            "0.9秒直线冲锋预警后向英雄所在横线突进。",
            "冲锋落点产生震地环；接近城门时造成额外耐久伤害。",
        ],
    },
    BossDef {
        id: "abyssal_leviathan",
        name: "深渊潮兽",
        spawn_seconds: 10 * 60,
        hp: 28_000.0,
        gate_damage: 180,
        modifiers: BossModifiers {
            max_hp_multiplier: 1.70,
            move_speed_multiplier: 0.60,
            ..BossModifiers::NEUTRAL
        },
        mechanics: &[
            "生命上限提高70%、移动速度降低40%，形成耐久战。",
            "施放前大幅减速并显示环形潮汐预警。",
            "潮汐完成时恢复8%最大生命，要求稳定持续输出。",
        ],
    },
    BossDef {
        id: "dread_veil",
        name: "恐幕行者",
        spawn_seconds: 15 * 60,
        hp: 70_000.0,
        gate_damage: 240,
        modifiers: BossModifiers {
            damage_reduction: 0.15,
            elemental_resistance: 0.10,
            ..BossModifiers::NEUTRAL
        },
        mechanics: &[
            "常驻15%伤害减免与10%全元素抗性。",
            "在左右侧预警点间穿梭，周期改变队形与瞄准方向。",
            "每次位移后召来三名无奖励随从压迫城门。",
        ],
    },
    BossDef {
        id: "sun_crown_magus",
        name: "日冕术王",
        spawn_seconds: 20 * 60,
        hp: 160_000.0,
        gate_damage: 320,
        modifiers: BossModifiers {
            move_speed_multiplier: 1.30,
            attack_speed_multiplier: 1.50,
            ..BossModifiers::NEUTRAL
        },
        mechanics: &[
            "日冕相位提高移动和突破伤害，并向城门释放灼热脉冲。",
            "暗冕相位降低行动速度与伤害，同时恢复3.5%最大生命。",
            "每次相位切换均显示范围不同的环形预警。",
        ],
    },
    BossDef {
        id: "green_regent",
        name: "翠绿摄政",
        spawn_seconds: 25 * 60,
        hp: 420_000.0,
        gate_damage: 500,
        modifiers: BossModifiers::NEUTRAL,
        mechanics: &[
            "每7秒召来五名无奖励护从，让主战线持续承压。",
            "入场90秒后开始狂暴，之后每10秒伤害提高25%。",
            "每层狂暴同时提高6%移动速度，并以扩散环明确反馈。",
            "击败后进入60秒战果刻印挑战，而非直接结算胜利。",
        ],
    },
];
