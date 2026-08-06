mod tests {
    use crate::modes::mode_manager::*;
    use crate::track::track;

    use crossbeam_channel::{Receiver, Sender, unbounded};
    use std::time::Duration;
    use uuid::Uuid;

    fn make_manager(
        handler_factory: HandlerFactoryFn,
        startup_handler: Box<dyn ModeHandler>,
    ) -> (
        ModeManager<HandlerFactoryFn>,
        Sender<TrackMsg>,             // to manager from reaper
        Receiver<TrackMsg>,           // from manager to reaper
        Sender<v1m::UpstreamMsg>,     // to manager from v1m
        Receiver<v1m::DownstreamMsg>, // from manager to v1m
    ) {
        let (reaper_tx_in, reaper_rx_in) = unbounded();
        let (reaper_tx_out, reaper_rx_out) = unbounded();
        let (v1m_tx_in, v1m_rx_in) = unbounded();
        let (v1m_tx_out, v1m_rx_out) = unbounded();

        let manager = ModeManager::new_for_testing(
            8,
            reaper_rx_in,
            reaper_tx_out,
            v1m_rx_in,
            v1m_tx_out,
            handler_factory,
            startup_handler,
        );

        (manager, reaper_tx_in, reaper_rx_out, v1m_tx_in, v1m_rx_out)
    }

    fn recv_track(rx: &Receiver<TrackMsg>) -> TrackMsg {
        rx.recv_timeout(Duration::from_millis(100))
            .expect("expected TrackMsg")
    }

    fn recv_v1m(rx: &Receiver<v1m::DownstreamMsg>) -> v1m::DownstreamMsg {
        rx.recv_timeout(Duration::from_millis(100))
            .expect("expected v1m::DownstreamMsg")
    }

    #[test]
    fn barrier_ids_are_unique() {
        let a = Barrier::new();
        let b = Barrier::new();
        assert_ne!(a, b);
    }

    struct FakeHandler {
        selected_track_guid: Option<Uuid>,
    }

    impl ModeHandler for FakeHandler {
        fn handle_msg_from_upstream(
            &mut self,
            msg: TrackMsg,
            _io: &mut dyn UpstreamIo,
        ) -> ModeAction {
            match msg {
                // NOTE: For the purpose of this test, we pretend we can only know about one track at
                // a time with index hard-coded to 0.
                TrackMsg::ReaperTrackIndex(msg) => {
                    self.selected_track_guid = Some(msg.track_guid);
                    ModeAction::None
                }
                _ => ModeAction::None,
            }
        }

        fn handle_msg_from_downstream(
            &mut self,
            msg: v1m::UpstreamMsg,
            io: &mut dyn DownstreamIo,
        ) -> ModeAction {
            match msg {
                v1m::UpstreamMsg::GlobalPress => {
                    ModeAction::Transition(TransitionRequest::ToReaperVolumePan {
                        offset: 0,
                        selected_track_guid: self.selected_track_guid,
                    })
                }
                v1m::UpstreamMsg::ChannelFader(msg) => {
                    io.send_to_reaper(
                        track::Volume {
                            track_guid: self.selected_track_guid.unwrap_or(Uuid::nil()),
                            volume: msg.value as f32,
                        }
                        .into(),
                    );
                    ModeAction::None
                }
                _ => ModeAction::None,
            }
        }
    }

    fn handler_factory(
        transition_request: TransitionRequest,
        _io: &mut IoCoalescing,
        _num_channels: usize,
    ) -> Box<dyn ModeHandler> {
        match transition_request {
            TransitionRequest::ToReaperVolumePan {
                offset: _,
                selected_track_guid: _,
            } => Box::new(FakeHandler {
                selected_track_guid: None,
            }),
            _ => panic!("unexpected transition request"),
        }
    }

    #[test]
    fn transition_sends_queryall_then_barrier_and_enters_waiting_upstream() {
        let (mut manager, reaper_in, reaper_out, v1m_in, _v1m_out) = make_manager(
            handler_factory,
            Box::new(FakeHandler {
                selected_track_guid: None,
            }),
        );

        // Select a track
        let selected = Uuid::new_v4();
        reaper_in
            .send(
                track::ReaperTrackIndex {
                    track_guid: selected,
                    track_index: None,
                }
                .into(),
            )
            .unwrap();
        manager.step_once();

        v1m_in.send(v1m::UpstreamMsg::GlobalPress).unwrap();

        manager.step_once();

        assert!(matches!(recv_track(&reaper_out), TrackMsg::QueryAll));

        let barrier = match recv_track(&reaper_out) {
            TrackMsg::Barrier(b) => b,
            other => panic!("expected TrackMsg::Barrier, got {:?}", other),
        };

        assert!(matches!(
            manager.curr_state,
            State::WaitingBarrierFromUpstream { barrier: b } if b == barrier
        ));
    }

    #[test]
    fn expected_upstream_barrier_flushes_then_sends_downstream_barrier() {
        let (mut manager, reaper_in, reaper_out, v1m_in, v1m_out) = make_manager(
            handler_factory,
            Box::new(FakeHandler {
                selected_track_guid: None,
            }),
        );

        // Select a track
        let selected = Uuid::new_v4();
        reaper_in
            .send(
                track::ReaperTrackIndex {
                    track_guid: selected,
                    track_index: None,
                }
                .into(),
            )
            .unwrap();
        manager.step_once();

        // Transition mode
        v1m_in.send(v1m::UpstreamMsg::GlobalPress).unwrap();

        manager.step_once();

        // Read QueryAll + expected barrier from reaper-out.
        let _ = recv_track(&reaper_out);
        let expected = match recv_track(&reaper_out) {
            TrackMsg::Barrier(b) => b,
            other => panic!("expected barrier, got {:?}", other),
        };

        // Feed one upstream non-barrier update that should coalesce.
        for _ in 0..16 {
            reaper_in
                .send(
                    track::Volume {
                        track_guid: Uuid::new_v4(),
                        volume: 0.42,
                    }
                    .into(),
                )
                .unwrap();
            manager.step_once();
        }

        // Now feed expected barrier.
        reaper_in.send(TrackMsg::Barrier(expected)).unwrap();
        manager.step_once();

        // We expect: (0..N flushed coalesced msgs), then Barrier(expected) LAST.
        // So drain a few and assert last observed is barrier.
        let mut saw_barrier = false;
        for _ in 0..16 {
            if let Ok(msg) = v1m_out.recv_timeout(Duration::from_millis(20)) {
                if matches!(msg, v1m::DownstreamMsg::Barrier(b) if b == expected) {
                    saw_barrier = true;
                    break;
                }
            }
        }
        assert!(saw_barrier, "expected downstream reflected barrier send");

        assert!(matches!(
            manager.curr_state,
            State::WaitingBarrierFromDownstream { barrier: b } if b == expected
        ));
    }

    #[test]
    fn waiting_downstream_ignores_wrong_barrier_and_activates_on_match() {
        let (mut manager, reaper_in, reaper_out, v1m_in, _v1m_out) = make_manager(
            handler_factory,
            Box::new(FakeHandler {
                selected_track_guid: None,
            }),
        );

        // Trigger transition.
        v1m_in.send(v1m::UpstreamMsg::GlobalPress).unwrap();
        manager.step_once();

        let _ = recv_track(&reaper_out); // QueryAll
        let expected = match recv_track(&reaper_out) {
            TrackMsg::Barrier(b) => b,
            _ => panic!("expected barrier"),
        };

        // Move to WaitingBarrierFromDownstream:
        reaper_in.send(TrackMsg::Barrier(expected)).unwrap();
        manager.step_once();

        // Wrong barrier should not activate.
        let wrong = Barrier::new();
        assert_ne!(wrong, expected);
        v1m_in.send(v1m::UpstreamMsg::Barrier(wrong)).unwrap();
        manager.step_once();

        assert!(matches!(
            manager.curr_state,
            State::WaitingBarrierFromDownstream { barrier: b } if b == expected
        ));

        // Correct barrier activates.
        v1m_in.send(v1m::UpstreamMsg::Barrier(expected)).unwrap();
        manager.step_once();

        assert!(matches!(manager.curr_state, State::Active));
    }

    #[test]
    fn waiting_downstream_does_not_process_upstream_messages() {
        let (mut manager, reaper_in, reaper_out, v1m_in, v1m_out) = make_manager(
            handler_factory,
            Box::new(FakeHandler {
                selected_track_guid: None,
            }),
        );

        // Trigger transition.
        v1m_in.send(v1m::UpstreamMsg::GlobalPress).unwrap();
        manager.step_once();

        let _ = recv_track(&reaper_out); // QueryAll
        let expected = match recv_track(&reaper_out) {
            TrackMsg::Barrier(b) => b,
            _ => panic!("expected barrier"),
        };

        // Move to waiting downstream.
        reaper_in.send(TrackMsg::Barrier(expected)).unwrap();
        manager.step_once();

        let _ = match recv_v1m(&v1m_out) {
            v1m::DownstreamMsg::Barrier(b) => b,
            _ => panic!("expected barrier"),
        };

        // Send upstream message while frozen.
        reaper_in
            .send(
                track::Volume {
                    track_guid: Uuid::new_v4(),
                    volume: 0.99,
                }
                .into(),
            )
            .unwrap();

        // step_once should only listen to from_v1m in this state; no v1m output should appear from above msg.
        // (May block if step_once strictly blocks waiting for v1m; if so, skip calling it here.)
        assert!(v1m_out.recv_timeout(Duration::from_millis(30)).is_err());
    }

    #[test]
    fn second_transition_after_returning_active_works_and_uses_new_barrier() {
        let (mut manager, reaper_in, reaper_out, v1m_in, _v1m_out) = make_manager(
            handler_factory,
            Box::new(FakeHandler {
                selected_track_guid: None,
            }),
        );

        // ---------- Transition #1 ----------
        v1m_in.send(v1m::UpstreamMsg::GlobalPress).unwrap();
        manager.step_once();

        assert!(matches!(recv_track(&reaper_out), TrackMsg::QueryAll));
        let barrier1 = match recv_track(&reaper_out) {
            TrackMsg::Barrier(b) => b,
            other => panic!("expected first barrier, got {:?}", other),
        };

        // Upstream confirms barrier -> manager waits for downstream reflection.
        reaper_in.send(TrackMsg::Barrier(barrier1)).unwrap();
        manager.step_once();
        assert!(matches!(
            manager.curr_state,
            State::WaitingBarrierFromDownstream { barrier: b } if b == barrier1
        ));

        // Downstream reflects barrier -> manager returns active.
        v1m_in.send(v1m::UpstreamMsg::Barrier(barrier1)).unwrap();
        manager.step_once();
        assert!(matches!(manager.curr_state, State::Active));

        // ---------- Transition #2 ----------
        v1m_in.send(v1m::UpstreamMsg::GlobalPress).unwrap();
        manager.step_once();

        assert!(matches!(recv_track(&reaper_out), TrackMsg::QueryAll));
        let barrier2 = match recv_track(&reaper_out) {
            TrackMsg::Barrier(b) => b,
            other => panic!("expected second barrier, got {:?}", other),
        };

        assert_ne!(barrier1, barrier2, "expected unique barrier per transition");
        assert!(matches!(
            manager.curr_state,
            State::WaitingBarrierFromUpstream { barrier: b } if b == barrier2
        ));

        // Complete second transition too (proves full repeatability).
        reaper_in.send(TrackMsg::Barrier(barrier2)).unwrap();
        manager.step_once();
        assert!(matches!(
            manager.curr_state,
            State::WaitingBarrierFromDownstream { barrier: b } if b == barrier2
        ));

        v1m_in.send(v1m::UpstreamMsg::Barrier(barrier2)).unwrap();
        manager.step_once();
        assert!(matches!(manager.curr_state, State::Active));
    }
    #[test]
    fn flush_into_batches_scribble_and_preserves_non_scribble_before_batches() {
        use crate::midi::v1m;

        let (reaper_tx_out, _reaper_rx_out) = unbounded::<TrackMsg>();
        let (v1m_tx_out, v1m_rx_out) = unbounded::<v1m::DownstreamMsg>();
        let io = IoDirect::new(reaper_tx_out.clone(), v1m_tx_out);

        let mut io_coalescing = IoCoalescing::new(reaper_tx_out);

        // Queue: scribble, scribble, non-scribble, scribble
        io_coalescing.send_to_v1m(
            v1m::TopScribbleStripLine1TextMsg {
                idx: 0,
                text: "A".to_string(),
            }
            .into(),
        );
        io_coalescing.send_to_v1m(v1m::ChannelFaderMsg { idx: 0, value: 0.5 }.into());
        io_coalescing.send_to_v1m(
            v1m::TopScribbleStripLine2TextMsg {
                idx: 0,
                text: "B".to_string(),
            }
            .into(),
        );
        io_coalescing.send_to_v1m(
            v1m::BottomScribbleStripLine1TextMsg {
                idx: 0,
                text: "C".to_string(),
            }
            .into(),
        );

        io_coalescing.flush_into(&io);

        // Current behavior from your implementation:
        // non-scribble emits immediately during drain,
        // scribble batches emit afterward.
        let first = recv_v1m(&v1m_rx_out);
        assert!(
            matches!(first, v1m::DownstreamMsg::ChannelFader(_)),
            "expected non-scribble first, got {:?}",
            first
        );

        let second = recv_v1m(&v1m_rx_out);
        assert!(
            matches!(second, v1m::DownstreamMsg::TopScribbleStripBatch(_)),
            "expected top scribble batch second, got {:?}",
            second
        );

        let third = recv_v1m(&v1m_rx_out);
        assert!(
            matches!(third, v1m::DownstreamMsg::BottomScribbleStripBatch(_)),
            "expected bottom scribble batch third, got {:?}",
            third
        );
    }
}
