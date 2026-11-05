// Excerpt of actual compiled old-order regression, not a standalone crate.
    #[test]
    fn paused_purchase_updates_hud_and_market_wallet_in_the_purchase_frame() {
        let mut app = test_app(MARKET_UNLOCK_AT + 1.0);
        // Force the old legal consumer-before-purchase order for the regression.
        app.add_systems(Update, super::super::ui::update_resource_counters.before(handle_market_actions));
        let hud = app.world_mut().spawn((
            super::super::ui::HudValue::Kills, Text::new("0"),
        )).id();
        app.world_mut().resource_mut::<MarketEffects>().market_discount = 0.20;
        app.world_mut().resource_mut::<MarketState>().offers[0].item = MarketItem::HardenedPlate;
        app.world_mut().resource_mut::<GameSession>().kills = 424;
        tap_b(&mut app);
        assert_eq!(app.world().get::<Text>(hud).unwrap().0, "424");
        for mut interaction in app.world_mut()
            .query_filtered::<&mut Interaction, With<presentation::PreviewBuy>>()
            .iter_mut(app.world_mut())
        {
            *interaction = Interaction::Pressed;
        }
        app.update();
        assert_eq!(app.world().resource::<GameSession>().kills, 280);
        assert!(app.world().resource::<MarketEffects>().owns(MarketItem::HardenedPlate));
        assert_ne!(app.world().resource::<MarketState>().offers[0].item, MarketItem::HardenedPlate);
        assert!(market_feedback(&mut app).contains("Bought Hardened Plate"));
        let wallet = app.world_mut().query_filtered::<&Text, With<MarketWalletLabel>>()
            .single(app.world()).unwrap().0.clone();
        assert!(wallet.contains("KILLS 0280"), "{wallet}");
        assert_eq!(app.world().get::<Text>(hud).unwrap().0, "280",
            "HUD must show the same actual post-purchase balance as the market in this frame");
        assert!(app.world().resource::<GameSession>().paused);
    }

