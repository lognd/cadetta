+++
id = "01M4SWMDS8MDHBS4T9JX1R3K6N"
title = "Push guard R010 and R006"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:44:12Z"
updated = "2026-10-17T06:50:32Z"
persona = "design engineer"
capability = "see what a push would overwrite"
idempotency_key = "replan14:sw-guard"
labels = ["creates:adapters/solidworks/src/Cadetta.SolidWorks.Core/Guard/**", "creates:adapters/solidworks/src/Cadetta.SolidWorks.Com/Guard/**", "creates:adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Guard/**"]
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.Core/Guard/**", "adapters/solidworks/src/Cadetta.SolidWorks.Com/Guard/**", "adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Guard/**", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Push/IPushDocument.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Push/NameMangler.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Push/NativeKind.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Push/PushDiagnostics.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Push/PushNumber.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Push/PushOptions.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Push/PushOutcome.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Push/PushService.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Push/ToleranceSpec.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Com/Push/SwComPushDocument.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Tests/Push/FakePushDocument.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Tests/Push/PushServiceTests.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Tests/Guard/**", "changelog.d/01M4SPWHPHR3VVX5NCSXF4502V.added.md", "changelog.d/01M4SPWHRZF9R1KGA3GV2AZF30.added.md", "changelog.d/01M4SPWHVJVHRD0KWP8ESJD81M.added.md", "changelog.d/01M4SPWNBYWT610BHC6GNSP35P.added.md", "changelog.d/01M4SPWNEF5D7VN10KTWKPM465.added.md", "changelog.d/01M4SPWNHPTRETQBYCM6EA2GEV.added.md", "changelog.d/01M4SPWNNDNT60Z38K24AEKVAE.added.md", "changelog.d/01M4SPWNR85DB2J5CGXZK9NP4M.added.md", "changelog.d/01M4SPWNVX14PTZ4B19436P66Q.added.md", "changelog.d/01M4SPWNYX632H3Q6WEGN0SM0R.added.md", "changelog.d/01M4SPWP1JK8XH8EKXK71P7EFH.added.md", "changelog.d/01M4SPWP4HDVWPMA4AFJ65KWPN.added.md", "changelog.d/01M4SPWP7B9ANMSNWB9SYD2T0Q.added.md", "changelog.d/01M4SPWPAJ4ZGEGGVF63ZY806Z.added.md", "changelog.d/01M4SPWPCW1C0QC67GZG99EHTZ.added.md", "changelog.d/01M4SPWPG1WBAHFPZ19ZV3BAPJ.added.md", "changelog.d/01M4SPWPKGXD3TJ78NPJH8Q9VZ.added.md", "changelog.d/01M4SPWPPPSEEFAHYTY5H3YD54.added.md", "changelog.d/01M4SPWPT5XR3QCQJ7M1XP07MP.added.md", "changelog.d/01M4SPWPXSG1YKHGF4KV5G43BT.added.md"]

[[links]]
kind = "blocked-by"
target = "01M4SV2TR8ENTA87J1DDKCF816"

[[links]]
kind = "blocked-by"
target = "01M4SWMDFARQR4Q9HCBERB21PP"

[[acceptance]]
text = "Given a read-only file, when pushing, then R010 is reported; given an existing equation, then R006 lists it before replacing (adapters.md 4)"
bound = false
+++
