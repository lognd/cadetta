# SOLIDWORKS API surface (docs/spec/solidworks.md section 4)

Exercised with `cadetta-solidworks --probe-api` against SOLIDWORKS 2025
(RevisionNumber 33.3.0, interop 33.3.0.92) on host `nova` (Windows ARM64,
x64 emulation), attached to a running instance in console session 1.
All rows below came back `ok`; no API was found missing, so no spec
tickets were filed.

| API | Call | Result | Detail |
|---|---|---|---|
| session | `ISldWorks.RevisionNumber()` | ok | 33.3.0 |
| session | `ISldWorks.GetUserPreferenceStringValue(swDefaultTemplatePart)` | ok | C:\Users\logan\Documents\Class Files\EML2322L\Templates\Part.prtdot |
| part | `ISldWorks.NewDocument + SketchManager.CreateCornerRectangle + FeatureManager.FeatureExtrusion2` | ok | feature Boss-Extrude1 (Extrusion) in C:\Users\logan\Documents\Class Files\EML2322L\Templates\Part.prtdot |
| equation manager | `IModelDoc2.GetEquationMgr().Add2(-1, "\"PROBE_T\" = 6mm", true)` | ok | index 0, Equation="PROBE_T" = 6mm, Value=6, GlobalVariable=true |
| equation manager | `IEquationMgr.Add2(-1, "\"D1@Boss-Extrude1\" = \"PROBE_T\"", true) (ref mode)` | ok | index 1, Equation="D1@Boss-Extrude1" = "PROBE_T", dimension=0.006 m, Status=1 |
| equation manager | `IEquationMgr.set_Equation(0, "\"PROBE_T\" = 8mm") then ForceRebuild3` | ok | dimension=0.008 m, GetCount=2 |
| IDimension | `IModelDoc2.Parameter("D1@Boss-Extrude1") as Dimension; Name, FullName, SystemValue, DrivenState` | ok | Name=D1, FullName=D1@Boss-Extrude1@Part8.Part, SystemValue=0.008 m, DrivenState=1 |
| IDimensionTolerance | `Tolerance.Type = swTolSYMMETRIC; SetValues(0.0001, 0.0001)` | ok | Type=4, min=0.0001, max=0.0001 |
| IDimensionTolerance | `Tolerance.Type = swTolBILAT; SetValues(-0.0001, 0.0002)` | ok | Type=2, min=-0.0001, max=0.0002 |
| IDimensionTolerance | `Tolerance.Type = swTolLIMIT; SetValues(-0.0001, 0.0002)` | ok | Type=3, min=-0.0001, max=0.0002 |
| IDimensionTolerance | `Tolerance.Type = swTolFIT; FitType = swFitCLEARANCE; SetFitValues("H7", "g6")` | ok | Type=7, min=0, max=1.4999999999999999E-05, FitType=1, hole=H7, shaft=g6 |
| IDimensionTolerance | `Tolerance.Type = swTolNONE` | ok | Type=0, min=0, max=1.4999999999999999E-05 |
| custom properties | `Extension.CustomPropertyManager[""].Add3 then Get5` | ok | Add3=0, Get5 value=v1, resolved=v1, count=1 |
| custom properties | `Extension.CustomPropertyManager[configName].Add3 then Get5 (configuration-specific)` | ok | config=Default, value=c1, absent at file level |
| coordinate system | `IFeatureManager.CreateCoordinateSystemUsingNumericalValues(true, 0.01, 0.02, 0.03, true, 0, 0, 0)` | ok | feature PROBE_CS type CoordSys |
| coordinate system | `IModelDocExtension.GetCoordinateSystemTransformByName("PROBE_CS").ArrayData` | ok | ArrayData=[1, 0, 0, 0, 1, 0, 0, 0, 1, 0.01, 0.02, 0.03, 1, 0, 0, 0] |
| coordinate system | `CreateCoordinateSystemUsingNumericalValues(true, 0, 0, 0, true, 0, 0, 1.5707963267949) (rotation convention)` | ok | rotate 90deg about z: ArrayData=[6.123233995736766E-17, 1, 0, -1, 6.123233995736766E-17, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0] |
| rebuild | `IModelDoc2.ForceRebuild3(false) returns true on a healthy model` | ok | ForceRebuild3=True, errors=0 |
| rebuild | `IModelDoc2.InsertFeatureShell(0.001, false); IEquationMgr.set_Equation(shell dim = 50mm); ForceRebuild3; walk IFeature.GetErrorCode2` | ok | shell=Shell1, healthy errors=0, bad: ForceRebuild3=False, dim=0.05, errors=[Shell1 code 1], after restore 0 |
| part | `IModelDocExtension.SaveAs3` | ok | SaveAs3 ok, errors=0, warnings=0 |
| component | `ISldWorks.NewDocument(assembly template); IAssemblyDoc.AddComponent5(path, 0, "", false, "", 0, 0, 0)` | ok | Name2=Probe-1, PathName=C:\src\probe\Probe.SLDPRT |
| component | `IComponent2.Transform2 (get) .ArrayData` | ok | ArrayData=[1, 0, 0, 0, 1, 0, 0, 0, 1, -0.02, -0.015, -0.004, 1, 0, 0, 0] |
| component | `IMathUtility.CreateTransform(double[16]); IComponent2.Transform2 = t; read back` | ok | ArrayData=[1, 0, 0, 0, 1, 0, 0, 0, 1, 0.05, 0.06, 0.07, 1, 0, 0, 0] |
| component | `IComponent2.ComponentReference set and get` | ok | ComponentReference=PROBE_REF |
| rebuild | `assembly IModelDoc2.ForceRebuild3(false)` | ok | ForceRebuild3=True |
| component | `ISldWorks.CloseDoc (assembly, unsaved)` | ok | closed |
| part | `ISldWorks.CloseDoc` | ok | closed |

## Findings

- Equations silently reject non-positive or huge dimension values (no
  rebuild error, the old value stays). A real rebuild error needs a model
  that cannot be built, e.g. a shell thicker than the part
  (`Shell1 code 1`, `ForceRebuild3` false).
- Feature and plane names are locale/template dependent. Find features by
  type name (`RefPlane`, `ProfileFeature`, `Shell`), not by name; derive
  dimension names (`D1@<feature>`) from the found feature.
- `MathTransform.ArrayData` is 16 doubles: rotation 0-8, translation 9-11,
  scale 12. The component `Transform2` origin carries the part's own offset.
- Rebuild errors are listed by walking `FirstFeature`/`GetNextFeature` and
  calling `GetErrorCode2(out warn)`.
- COM attach works only from the same logon session as SOLIDWORKS (see
  DEVELOPING.md).
