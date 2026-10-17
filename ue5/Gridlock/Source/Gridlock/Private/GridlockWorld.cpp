#include "GridlockWorld.h"

#include "Camera/CameraComponent.h"
#include "Components/PointLightComponent.h"
#include "Components/SpotLightComponent.h"
#include "Components/StaticMeshComponent.h"
#include "Engine/StaticMesh.h"
#include "Framework/Application/SlateApplication.h"
#include "GameFramework/InputSettings.h"
#include "GameFramework/PlayerController.h"
#include "GameFramework/SpringArmComponent.h"
#include "GenericPlatform/IInputInterface.h"
#include "InputCoreTypes.h"
#include "Kismet/GameplayStatics.h"
#include "Materials/MaterialInstanceDynamic.h"
#include "ProceduralMeshComponent.h"
#include "UObject/ConstructorHelpers.h"

namespace
{
	FVector V3(const float* P) { return FVector(P[0], P[1], P[2]); }
	FQuat Q4(const float* Q) { return FQuat(Q[0], Q[1], Q[2], Q[3]); }
	FTransform ToTransform(const GlPose& P) { return FTransform(Q4(P.rot), V3(P.pos)); }
	FLinearColor Rgb(uint32 C) { return FLinearColor(FColor((C >> 16) & 255, (C >> 8) & 255, C & 255)); }

	// Prop kinds from gridlock.h (GlPropView.kind).
	enum EProp { Lamp = 0, Sign, SignalPole, Bollard, Hydrant, TrashCan, Cone, Bench, Barrier, Mailbox, Newsstand, Planter };

	const FLinearColor LampColors[4] = { FLinearColor(1.0f, 0.05f, 0.03f), FLinearColor(1.0f, 0.55f, 0.0f), FLinearColor(0.1f, 1.0f, 0.55f), FLinearColor::Black };
}

AGridlockWorld::AGridlockWorld()
{
	PrimaryActorTick.bCanEverTick = true;
	PrimaryActorTick.TickGroup = TG_PrePhysics;

	Root = CreateDefaultSubobject<USceneComponent>(TEXT("Root"));
	RootComponent = Root;
	CityMesh = CreateDefaultSubobject<UProceduralMeshComponent>(TEXT("City"));
	CityMesh->SetupAttachment(Root);
	CityMesh->bUseAsyncCooking = true;

	CameraArm = CreateDefaultSubobject<USpringArmComponent>(TEXT("CameraArm"));
	CameraArm->SetupAttachment(Root);
	CameraArm->TargetArmLength = 650.0f;
	CameraArm->SocketOffset = FVector(0, 0, 160);
	CameraArm->bEnableCameraLag = true;
	CameraArm->CameraLagSpeed = 8.0f;
	CameraArm->bEnableCameraRotationLag = true;
	CameraArm->CameraRotationLagSpeed = 6.0f;
	CameraArm->bDoCollisionTest = false;
	Camera = CreateDefaultSubobject<UCameraComponent>(TEXT("Camera"));
	Camera->SetupAttachment(CameraArm);
	Camera->FieldOfView = 70.0f;

	HeadlightL = CreateDefaultSubobject<USpotLightComponent>(TEXT("HeadlightL"));
	HeadlightR = CreateDefaultSubobject<USpotLightComponent>(TEXT("HeadlightR"));
	for (USpotLightComponent* L : { HeadlightL.Get(), HeadlightR.Get() })
	{
		L->SetupAttachment(Root);
		L->SetIntensityUnits(ELightUnits::Candelas);
		L->SetIntensity(9000.0f);
		L->SetAttenuationRadius(6000.0f);
		L->SetInnerConeAngle(14.0f);
		L->SetOuterConeAngle(32.0f);
		L->SetLightColor(FLinearColor(1.0f, 0.95f, 0.88f));
		L->SetVisibility(false);
	}

	static ConstructorHelpers::FObjectFinder<UStaticMesh> CubeF(TEXT("/Engine/BasicShapes/Cube.Cube"));
	static ConstructorHelpers::FObjectFinder<UStaticMesh> CylF(TEXT("/Engine/BasicShapes/Cylinder.Cylinder"));
	static ConstructorHelpers::FObjectFinder<UStaticMesh> SphF(TEXT("/Engine/BasicShapes/Sphere.Sphere"));
	Cube = CubeF.Object;
	Cylinder = CylF.Object;
	Sphere = SphF.Object;
}

void AGridlockWorld::BeginPlay()
{
	Super::BeginPlay();
	GlConfig Cfg;
	Cfg.seed = Seed;
	Cfg.traffic = (uint32)FMath::Max(0, TrafficCount);
	Cfg.police = (uint32)FMath::Max(0, PoliceCount);
	Cfg.start_hour = StartHour;
	Cfg.player_class = (uint32)PlayerVehicle;
	Cfg.coords = GL_COORDS_UNREAL;
	Cfg.props = bSpawnProps ? 1u : 0u;
	Game = gl_game_create(&Cfg);
	if (!Game)
	{
		UE_LOG(LogTemp, Error, TEXT("Gridlock: gl_game_create failed"));
		return;
	}
	UE_LOG(LogTemp, Log, TEXT("Gridlock %s started"), UTF8_TO_TCHAR(gl_version()));
	BuildCity();
	if (bPlayerDrives)
	{
		if (APlayerController* PC = UGameplayStatics::GetPlayerController(this, 0))
		{
			PC->SetViewTarget(this);
		}
	}
}

void AGridlockWorld::EndPlay(const EEndPlayReason::Type Reason)
{
	if (Game)
	{
		if (FSlateApplication::IsInitialized())
		{
			if (IInputInterface* Input = FSlateApplication::Get().GetInputInterface())
			{
				Input->SetForceFeedbackChannelValues(0, FForceFeedbackValues());
			}
		}
		gl_game_destroy(Game);
		Game = nullptr;
	}
	Super::EndPlay(Reason);
}

void AGridlockWorld::AddSection(UProceduralMeshComponent* Mesh, int32 Index, const TArray<FVector>& V, TArray<int32> T, const TArray<FVector>& N, const TArray<FVector2D>& UV, const TArray<FColor>& Col, bool bCollision, UMaterialInterface* Material)
{
	if (bReverseWinding)
	{
		for (int32 i = 0; i + 2 < T.Num(); i += 3)
		{
			Swap(T[i + 1], T[i + 2]);
		}
	}
	Mesh->CreateMeshSection(Index, V, T, N, UV, Col, TArray<FProcMeshTangent>(), bCollision);
	if (Material)
	{
		Mesh->SetMaterial(Index, Material);
	}
}

void AGridlockWorld::FillSection(UProceduralMeshComponent* Mesh, int32 Section, const GlMesh& M, UMaterialInterface* Material, bool bUpdateOnly)
{
	TArray<FVector> Verts;
	TArray<FVector> Normals;
	TArray<FVector2D> Uvs;
	Verts.SetNumUninitialized(M.vertex_count);
	Normals.SetNumUninitialized(M.vertex_count);
	Uvs.SetNumUninitialized(M.vertex_count);
	for (uint32 i = 0; i < M.vertex_count; ++i)
	{
		Verts[i] = V3(M.positions + i * 3);
		Normals[i] = V3(M.normals + i * 3);
		Uvs[i] = FVector2D(M.uvs[i * 2], M.uvs[i * 2 + 1]);
	}
	if (bUpdateOnly)
	{
		Mesh->UpdateMeshSection(Section, Verts, Normals, Uvs, TArray<FColor>(), TArray<FProcMeshTangent>());
		return;
	}
	TArray<int32> Tris;
	Tris.SetNumUninitialized(M.index_count);
	for (uint32 i = 0; i < M.index_count; ++i)
	{
		Tris[i] = (int32)M.indices[i];
	}
	AddSection(Mesh, Section, Verts, MoveTemp(Tris), Normals, Uvs, TArray<FColor>(), false, Material);
}

void AGridlockWorld::BuildCity()
{
	int32 Section = 0;
	// Road slab: one big quad under everything (blocks sit 15 cm above it).
	{
		TArray<GlBlock> Blocks;
		Blocks.SetNumZeroed(gl_city_blocks(Game, nullptr, 0));
		gl_city_blocks(Game, Blocks.GetData(), Blocks.Num());
		FBox Bounds(ForceInit);
		for (const GlBlock& B : Blocks)
		{
			for (uint32 i = 0; i < B.point_count; ++i)
			{
				Bounds += V3(B.outline[i]);
			}
		}
		const FVector Mn = Bounds.Min, Mx = Bounds.Max;
		TArray<FVector> V = { FVector(Mn.X, Mn.Y, 0), FVector(Mx.X, Mn.Y, 0), FVector(Mx.X, Mx.Y, 0), FVector(Mn.X, Mx.Y, 0) };
		TArray<int32> T = { 0, 2, 1, 0, 3, 2 };
		TArray<FVector> N = { FVector::UpVector, FVector::UpVector, FVector::UpVector, FVector::UpVector };
		TArray<FVector2D> UV = { FVector2D(Mn.X, Mn.Y) / 100.0, FVector2D(Mx.X, Mn.Y) / 100.0, FVector2D(Mx.X, Mx.Y) / 100.0, FVector2D(Mn.X, Mx.Y) / 100.0 };
		AddSection(CityMesh, Section++, V, T, N, UV, TArray<FColor>(), true, RoadMaterial);

		// Raised blocks (sidewalk/lots): top fan + kerb walls, one section each for sidewalks and verges.
		for (int32 Pass = 0; Pass < 2; ++Pass)
		{
			TArray<FVector> BV;
			TArray<int32> BT;
			TArray<FVector> BN;
			TArray<FVector2D> BUV;
			for (const GlBlock& B : Blocks)
			{
				if ((B.outer != 0) != (Pass == 1))
				{
					continue;
				}
				const int32 Base = BV.Num();
				FVector C(0);
				for (uint32 i = 0; i < B.point_count; ++i)
				{
					C += V3(B.outline[i]);
				}
				C /= FMath::Max(1u, B.point_count);
				C.Z = B.height;
				BV.Add(C);
				BN.Add(FVector::UpVector);
				BUV.Add(FVector2D(C.X, C.Y) / 100.0);
				for (uint32 i = 0; i < B.point_count; ++i)
				{
					FVector P = V3(B.outline[i]);
					P.Z = B.height;
					BV.Add(P);
					BN.Add(FVector::UpVector);
					BUV.Add(FVector2D(P.X, P.Y) / 100.0);
				}
				const int32 Count = (int32)B.point_count;
				for (int32 i = 0; i < Count; ++i)
				{
					const int32 A = Base + 1 + i, Bb = Base + 1 + (i + 1) % Count;
					BT.Append({ Base, Bb, A });
				}
				// Kerb faces.
				for (uint32 i = 0; i < B.point_count; ++i)
				{
					const FVector P0 = V3(B.outline[i]), P1 = V3(B.outline[(i + 1) % B.point_count]);
					const FVector Out = FVector::CrossProduct(P1 - P0, FVector::UpVector).GetSafeNormal();
					const int32 K = BV.Num();
					BV.Append({ P0, P1, P1 + FVector(0, 0, B.height), P0 + FVector(0, 0, B.height) });
					BN.Append({ Out, Out, Out, Out });
					const float L = (P1 - P0).Size() / 100.0f;
					BUV.Append({ FVector2D(0, 0), FVector2D(L, 0), FVector2D(L, 0.15f), FVector2D(0, 0.15f) });
					BT.Append({ K, K + 2, K + 1, K, K + 3, K + 2 });
				}
			}
			AddSection(CityMesh, Section++, BV, BT, BN, BUV, TArray<FColor>(), true, Pass == 0 ? SidewalkMaterial : GrassMaterial);
		}
	}
	// Buildings: boxes with metre-scale UVs.
	{
		TArray<GlBuilding> B;
		B.SetNumZeroed(gl_city_buildings(Game, nullptr, 0));
		gl_city_buildings(Game, B.GetData(), B.Num());
		TArray<FVector> V;
		TArray<int32> T;
		TArray<FVector> N;
		TArray<FVector2D> UV;
		TArray<FColor> Col;
		for (const GlBuilding& G : B)
		{
			const FVector Base = V3(G.base), Size = V3(G.size);
			if (Size.Z < 50.0f)
			{
				continue; // parks/plazas
			}
			const FVector H(Size.X * 0.5f, Size.Y * 0.5f, 0);
			const FColor Tint = FLinearColor::MakeFromHSV8((uint8)(G.hue * 255.0f), 40, 200).ToFColor(true);
			const FVector Corners[4] = { Base + FVector(-H.X, -H.Y, 0), Base + FVector(H.X, -H.Y, 0), Base + FVector(H.X, H.Y, 0), Base + FVector(-H.X, H.Y, 0) };
			for (int32 s = 0; s < 4; ++s)
			{
				const FVector A = Corners[s], Bc = Corners[(s + 1) % 4];
				const FVector Out = FVector::CrossProduct(Bc - A, FVector::UpVector).GetSafeNormal();
				const int32 K = V.Num();
				V.Append({ A, Bc, Bc + FVector(0, 0, Size.Z), A + FVector(0, 0, Size.Z) });
				N.Append({ Out, Out, Out, Out });
				const float W = (Bc - A).Size() / 100.0f, Hm = Size.Z / 100.0f;
				UV.Append({ FVector2D(0, Hm), FVector2D(W, Hm), FVector2D(W, 0), FVector2D(0, 0) });
				Col.Append({ Tint, Tint, Tint, Tint });
				T.Append({ K, K + 2, K + 1, K, K + 3, K + 2 });
			}
			const int32 K = V.Num();
			for (int32 c = 0; c < 4; ++c)
			{
				V.Add(Corners[c] + FVector(0, 0, Size.Z));
				N.Add(FVector::UpVector);
				UV.Add(FVector2D(Corners[c].X, Corners[c].Y) / 100.0);
				Col.Add(Tint);
			}
			// Roof corners run clockwise seen from above: wind it the other way.
			T.Append({ K, K + 2, K + 1, K, K + 3, K + 2 });
		}
		AddSection(CityMesh, Section++, V, T, N, UV, Col, true, BuildingMaterial);
	}
	// Road markings: ribbons along each polyline (dashes are cut into 3 m on / 9 m off).
	{
		TArray<GlMarking> M;
		M.SetNumZeroed(gl_city_markings(Game, nullptr, 0));
		gl_city_markings(Game, M.GetData(), M.Num());
		TArray<FVector> V[2];
		TArray<int32> T[2];
		TArray<FVector> N[2];
		TArray<FVector2D> UV[2];
		TArray<float> Pts;
		for (int32 i = 0; i < M.Num(); ++i)
		{
			const GlMarking& Mk = M[i];
			Pts.SetNumZeroed(Mk.point_count * 3);
			gl_city_marking_points(Game, i, Pts.GetData(), Mk.point_count);
			const int32 Bin = Mk.kind == 2 ? 1 : 0;
			const float HalfW = Mk.width * 50.0f;
			auto Quad = [&](const FVector& A, const FVector& B, float Half)
			{
				const FVector Side = FVector::CrossProduct(B - A, FVector::UpVector).GetSafeNormal() * Half;
				const int32 K = V[Bin].Num();
				const FVector Lift(0, 0, 1.5f);
				V[Bin].Append({ A - Side + Lift, B - Side + Lift, B + Side + Lift, A + Side + Lift });
				N[Bin].Append({ FVector::UpVector, FVector::UpVector, FVector::UpVector, FVector::UpVector });
				UV[Bin].Append({ FVector2D(0, 0), FVector2D(1, 0), FVector2D(1, 1), FVector2D(0, 1) });
				T[Bin].Append({ K, K + 1, K + 2, K, K + 2, K + 3 });
			};
			if (Mk.kind == 4)
			{
				// Crosswalk: zebra stripes across the band between the two curb points.
				const FVector A = V3(&Pts[0]), B = V3(&Pts[3]);
				const FVector Along = (B - A).GetSafeNormal();
				const float Len = (B - A).Size();
				const FVector Across = FVector::CrossProduct(Along, FVector::UpVector) * (Mk.width * 50.0f);
				for (float d = 30.0f; d < Len - 30.0f; d += 110.0f)
				{
					const FVector P = A + Along * (d + 25.0f);
					Quad(P - Across, P + Across, 25.0f);
				}
				continue;
			}
			float Run = 0.0f;
			for (uint32 k = 0; k + 1 < Mk.point_count; ++k)
			{
				const FVector A = V3(&Pts[k * 3]), B = V3(&Pts[k * 3 + 3]);
				const float L = (B - A).Size();
				if (Mk.kind == 1)
				{
					// 3 m dash, 9 m gap.
					float t = 0.0f;
					while (t < L)
					{
						const float Phase = FMath::Fmod(Run + t, 1200.0f);
						const float Seg = Phase < 300.0f ? FMath::Min(300.0f - Phase, L - t) : FMath::Min(1200.0f - Phase, L - t);
						if (Phase < 300.0f && Seg > 1.0f)
						{
							Quad(A + (B - A) * (t / L), A + (B - A) * ((t + Seg) / L), HalfW);
						}
						t += FMath::Max(Seg, 1.0f);
					}
				}
				else
				{
					Quad(A, B, HalfW);
				}
				Run += L;
			}
		}
		for (int32 Bin = 0; Bin < 2; ++Bin)
		{
			AddSection(CityMesh, Section++, V[Bin], T[Bin], N[Bin], UV[Bin], TArray<FColor>(), false, Bin == 0 ? MarkingWhiteMaterial : MarkingYellowMaterial);
		}
	}
	// Street lights (closest N to the start get real lights; all lamps get emissive heads via props).
	{
		TArray<float> L;
		L.SetNumZeroed(gl_city_lamp_lights(Game, nullptr, 0) * 3);
		const int32 Count = gl_city_lamp_lights(Game, L.GetData(), L.Num() / 3);
		for (int32 i = 0; i < Count && StreetLights.Num() < MaxStreetLights; ++i)
		{
			UPointLightComponent* P = NewObject<UPointLightComponent>(this);
			P->SetupAttachment(Root);
			P->RegisterComponent();
			AddInstanceComponent(P);
			P->SetWorldLocation(V3(&L[i * 3]));
			P->SetIntensityUnits(ELightUnits::Candelas);
			P->SetIntensity(1800.0f);
			P->SetAttenuationRadius(2400.0f);
			P->SetLightColor(FLinearColor(1.0f, 0.78f, 0.52f));
			P->SetCastShadows(false);
			StreetLights.Add(P);
		}
	}
}

UMaterialInterface* AGridlockWorld::PaintSlot(uint32 Paint, FGridlockCarVisual& V) const
{
	switch (Paint)
	{
	case 0: return V.Paint ? static_cast<UMaterialInterface*>(V.Paint.Get()) : PaintMaterial.Get();
	case 1: return TrimMaterial;
	case 2: return GlassMaterial;
	case 3: return V.HeadLamps ? static_cast<UMaterialInterface*>(V.HeadLamps.Get()) : LampMaterial.Get();
	case 4: return V.TailLamps ? static_cast<UMaterialInterface*>(V.TailLamps.Get()) : LampMaterial.Get();
	case 5: return TyreMaterial;
	default: return RimMaterial;
	}
}

FGridlockCarVisual& AGridlockWorld::CarVisual(uint32 Id, uint32 ClassIndex)
{
	if (FGridlockCarVisual* Existing = Cars.Find(Id))
	{
		return *Existing;
	}
	FGridlockCarVisual& V = Cars.Add(Id);
	V.ClassIndex = ClassIndex;
	if (PaintMaterial)
	{
		V.Paint = UMaterialInstanceDynamic::Create(PaintMaterial, this);
	}
	if (LampMaterial)
	{
		V.HeadLamps = UMaterialInstanceDynamic::Create(LampMaterial, this);
		V.TailLamps = UMaterialInstanceDynamic::Create(LampMaterial, this);
		V.HeadLamps->SetVectorParameterValue(TEXT("Color"), FLinearColor(1.0f, 0.96f, 0.9f));
		V.TailLamps->SetVectorParameterValue(TEXT("Color"), FLinearColor(1.0f, 0.02f, 0.01f));
	}
	auto MakeMesh = [this](const TCHAR*)
	{
		UProceduralMeshComponent* M = NewObject<UProceduralMeshComponent>(this, NAME_None);
		M->SetupAttachment(Root);
		M->SetCollisionEnabled(ECollisionEnabled::NoCollision);
		M->RegisterComponent();
		AddInstanceComponent(M);
		return M;
	};
	const uint32 Count = gl_car_model_mesh_count(Game, ClassIndex);
	const uint32 PartCount = Count >= 3 ? Count - 3 : 0;
	V.Body = MakeMesh(TEXT("Body"));
	GlMesh M;
	if (gl_car_model_mesh(Game, ClassIndex, 0, &M))
	{
		FillSection(V.Body, 0, M, PaintSlot(M.paint, V));
	}
	for (uint32 p = 1; p <= PartCount; ++p)
	{
		UProceduralMeshComponent* Part = MakeMesh(TEXT("Part"));
		if (gl_car_model_mesh(Game, ClassIndex, p, &M))
		{
			FillSection(Part, 0, M, PaintSlot(M.paint, V));
		}
		V.Parts.Add(Part);
	}
	for (int32 w = 0; w < 4; ++w)
	{
		UProceduralMeshComponent* Wheel = MakeMesh(TEXT("Wheel"));
		if (gl_car_model_mesh(Game, ClassIndex, PartCount + 1, &M))
		{
			FillSection(Wheel, 0, M, TyreMaterial);
		}
		if (gl_car_model_mesh(Game, ClassIndex, PartCount + 2, &M))
		{
			FillSection(Wheel, 1, M, RimMaterial);
		}
		V.Wheels.Add(Wheel);
	}
	return V;
}

void AGridlockWorld::SyncCars()
{
	const uint32 N = gl_game_cars(Game, nullptr, 0);
	CarBuffer.SetNumZeroed(N);
	gl_game_cars(Game, CarBuffer.GetData(), N);
	for (auto& Pair : Cars)
	{
		Pair.Value.bSeen = false;
	}
	bool bHeadlights = false;
	for (const GlCarView& C : CarBuffer)
	{
		FGridlockCarVisual& V = CarVisual(C.id, C.class_index);
		V.bSeen = true;
		const FTransform T = ToTransform(C.pose);
		V.Body->SetWorldTransform(T);
		if (V.Paint)
		{
			V.Paint->SetVectorParameterValue(TEXT("Color"), Rgb(C.color));
		}
		// Crash deformation: re-fetch dented meshes when the revision changes.
		if (C.body_revision != 0 && C.body_revision != V.BodyRevision)
		{
			GlMesh M;
			if (gl_car_damaged_mesh(Game, C.id, 0, &M))
			{
				FillSection(V.Body, 0, M, nullptr, true);
			}
			for (int32 p = 0; p < V.Parts.Num(); ++p)
			{
				if (gl_car_damaged_mesh(Game, C.id, p + 1, &M))
				{
					FillSection(V.Parts[p], 0, M, nullptr, true);
				}
			}
			V.BodyRevision = C.body_revision;
		}
		for (int32 p = 0; p < V.Parts.Num(); ++p)
		{
			GlPose PartPose;
			V.Parts[p]->SetWorldTransform(gl_car_part_pose(Game, C.id, p + 1, &PartPose) ? ToTransform(PartPose) : T);
		}
		for (int32 w = 0; w < 4 && w < V.Wheels.Num(); ++w)
		{
			const bool bDetached = (C.flags >> (8 + w)) & 1;
			V.Wheels[w]->SetVisibility(!bDetached);
			V.Wheels[w]->SetWorldTransform(ToTransform(C.wheels[w]));
		}
		const bool bLow = C.lamps & 1, bHigh = (C.lamps >> 1) & 1;
		const float Brake = FMath::Max(C.brake, 0.0f);
		if (V.HeadLamps)
		{
			V.HeadLamps->SetScalarParameterValue(TEXT("Emissive"), bHigh ? 60.0f : bLow ? 30.0f : 0.5f);
		}
		if (V.TailLamps)
		{
			V.TailLamps->SetScalarParameterValue(TEXT("Emissive"), (bLow ? 8.0f : 0.5f) + Brake * 40.0f);
		}
		if (C.flags & 1)
		{
			PlayerTransform = T;
			bHeadlights = bLow || bHigh;
			const float Spread = bHigh ? 1.0f : 0.0f;
			// Lamps in car space (Unreal: forward is -Y because the sim's -Z forward maps to -Y).
			const FVector Fwd = T.TransformVectorNoScale(FVector(0, -1, 0));
			for (int32 s = 0; s < 2; ++s)
			{
				USpotLightComponent* L = s == 0 ? HeadlightL : HeadlightR;
				const FVector Pos = T.TransformPosition(FVector(s == 0 ? -65.0f : 65.0f, -230.0f, 70.0f));
				L->SetWorldLocationAndRotation(Pos, (Fwd - FVector(0, 0, 0.06f - 0.05f * Spread)).Rotation());
				L->SetOuterConeAngle(bHigh ? 38.0f : 30.0f);
				L->SetIntensity(bHigh ? 22000.0f : 9000.0f);
			}
		}
	}
	HeadlightL->SetVisibility(bHeadlights);
	HeadlightR->SetVisibility(bHeadlights);
	// Remove cars that despawned.
	for (auto It = Cars.CreateIterator(); It; ++It)
	{
		if (!It.Value().bSeen)
		{
			It.Value().Body->DestroyComponent();
			for (UProceduralMeshComponent* P : It.Value().Parts) P->DestroyComponent();
			for (UProceduralMeshComponent* W : It.Value().Wheels) W->DestroyComponent();
			It.RemoveCurrent();
		}
	}
}

void AGridlockWorld::SyncSignals()
{
	const uint32 N = gl_game_signals(Game, nullptr, 0);
	SignalBuffer.SetNumZeroed(N);
	gl_game_signals(Game, SignalBuffer.GetData(), N);
	// Three lamps per head (red, yellow, green), created lazily.
	while (SignalLamps.Num() < (int32)N * 3)
	{
		UStaticMeshComponent* S = NewObject<UStaticMeshComponent>(this);
		S->SetStaticMesh(Sphere);
		S->SetupAttachment(Root);
		S->SetCollisionEnabled(ECollisionEnabled::NoCollision);
		S->SetWorldScale3D(FVector(0.24f));
		S->RegisterComponent();
		AddInstanceComponent(S);
		UMaterialInstanceDynamic* Mid = LampMaterial ? UMaterialInstanceDynamic::Create(LampMaterial, this) : nullptr;
		if (Mid)
		{
			S->SetMaterial(0, Mid);
		}
		SignalLamps.Add(S);
		SignalMaterials.Add(Mid);
	}
	const float Blink = FMath::Fmod((float)GetWorld()->GetTimeSeconds(), 1.0f) < 0.5f ? 1.0f : 0.0f;
	for (uint32 i = 0; i < N; ++i)
	{
		const GlSignalView& S = SignalBuffer[i];
		const FVector Head = V3(S.head);
		for (int32 k = 0; k < 3; ++k)
		{
			UStaticMeshComponent* Lamp = SignalLamps[i * 3 + k];
			Lamp->SetWorldLocation(Head + FVector(0, 0, (1 - k) * 32.0f) + V3(S.facing) * 14.0f);
			if (UMaterialInstanceDynamic* Mid = SignalMaterials[i * 3 + k])
			{
				const bool bOn = S.ball == k && (!(S.flags & 1) || Blink > 0.5f);
				Mid->SetVectorParameterValue(TEXT("Color"), LampColors[k]);
				Mid->SetScalarParameterValue(TEXT("Emissive"), bOn ? 50.0f : 0.15f);
			}
		}
	}
}

void AGridlockWorld::SyncProps()
{
	const uint32 N = gl_game_props(Game, nullptr, 0);
	PropBuffer.SetNumZeroed(N);
	gl_game_props(Game, PropBuffer.GetData(), N);
	// Simple stand-in shapes (swap for real meshes by kind in a Blueprint subclass if desired).
	while (PropMeshes.Num() < (int32)N)
	{
		const GlPropView& P = PropBuffer[PropMeshes.Num()];
		UStaticMeshComponent* M = NewObject<UStaticMeshComponent>(this);
		FVector Scale(0.3f);
		UStaticMesh* Shape = Cylinder;
		switch (P.kind)
		{
		case Lamp: Scale = FVector(0.2f, 0.2f, 7.4f); break;
		case Sign: Scale = FVector(0.07f, 0.07f, 2.7f); break;
		case SignalPole: Scale = FVector(0.28f, 0.28f, 6.0f); break;
		case Bollard: Scale = FVector(0.22f, 0.22f, 1.0f); break;
		case Hydrant: Scale = FVector(0.3f, 0.3f, 0.8f); break;
		case TrashCan: Scale = FVector(0.56f, 0.56f, 1.0f); break;
		case Cone: Scale = FVector(0.36f, 0.36f, 0.72f); break;
		case Bench: Shape = Cube; Scale = FVector(1.8f, 0.45f, 0.5f); break;
		case Barrier: Shape = Cube; Scale = FVector(0.6f, 3.0f, 0.81f); break;
		case Mailbox: Shape = Cube; Scale = FVector(0.56f, 0.5f, 1.2f); break;
		case Newsstand: Shape = Cube; Scale = FVector(0.6f, 0.5f, 1.1f); break;
		default: Shape = Cube; Scale = FVector(1.2f, 1.2f, 0.7f); break;
		}
		M->SetStaticMesh(Shape);
		M->SetupAttachment(Root);
		M->SetCollisionEnabled(ECollisionEnabled::NoCollision);
		M->SetMaterial(0, TrimMaterial);
		M->RegisterComponent();
		AddInstanceComponent(M);
		// Engine basic shapes are 100 cm and centred; the sync lifts them onto the prop origin.
		M->SetRelativeScale3D(Scale);
		PropMeshes.Add(M);
	}
	for (uint32 i = 0; i < N; ++i)
	{
		const GlPropView& P = PropBuffer[i];
		UStaticMeshComponent* M = PropMeshes[i];
		const FTransform T = ToTransform(P.pose);
		const FVector Scale = M->GetRelativeScale3D();
		M->SetWorldLocationAndRotation(T.TransformPosition(FVector(0, 0, Scale.Z * 50.0f)), T.GetRotation());
		M->SetWorldScale3D(Scale);
	}
}

void AGridlockWorld::SyncPeds()
{
	const uint32 N = gl_game_peds(Game, nullptr, 0);
	PedBuffer.SetNumZeroed(N);
	gl_game_peds(Game, PedBuffer.GetData(), N);
	while (PedMeshes.Num() < (int32)N)
	{
		UStaticMeshComponent* M = NewObject<UStaticMeshComponent>(this);
		M->SetStaticMesh(Cylinder);
		M->SetupAttachment(Root);
		M->SetCollisionEnabled(ECollisionEnabled::NoCollision);
		M->RegisterComponent();
		AddInstanceComponent(M);
		PedMeshes.Add(M);
	}
	for (int32 i = 0; i < PedMeshes.Num(); ++i)
	{
		UStaticMeshComponent* M = PedMeshes[i];
		if (i >= (int32)N)
		{
			M->SetVisibility(false);
			continue;
		}
		const GlPedView& P = PedBuffer[i];
		const float H = P.height * 100.0f;
		const bool bDown = P.flags & 2;
		M->SetVisibility(true);
		M->SetWorldScale3D(bDown ? FVector(0.4f, 0.4f, 0.3f) * FVector(1, 1, H / 100.0f) : FVector(0.4f, 0.3f, H / 100.0f));
		const float Bob = (P.flags & 1) ? FMath::Abs(FMath::Sin(P.stride * PI)) * 3.0f : 0.0f;
		M->SetWorldLocationAndRotation(V3(P.pos) + FVector(0, 0, (bDown ? 15.0f : H * 0.5f) + Bob), FRotator(bDown ? 90.0f : 0.0f, FMath::RadiansToDegrees(-P.heading), 0));
	}
}

void AGridlockWorld::ReadInput(GlPad& Pad, TArray<FString>& Keys) const
{
	FMemory::Memzero(Pad);
	APlayerController* PC = UGameplayStatics::GetPlayerController(this, 0);
	if (!PC || !bPlayerDrives)
	{
		return;
	}
	Pad.connected = 1;
	Pad.axes[0] = PC->GetInputAnalogKeyState(EKeys::Gamepad_LeftX);
	Pad.axes[1] = PC->GetInputAnalogKeyState(EKeys::Gamepad_LeftY);
	Pad.axes[2] = PC->GetInputAnalogKeyState(EKeys::Gamepad_RightX);
	Pad.axes[3] = PC->GetInputAnalogKeyState(EKeys::Gamepad_RightY);
	Pad.axes[4] = PC->GetInputAnalogKeyState(EKeys::Gamepad_LeftTriggerAxis);
	Pad.axes[5] = PC->GetInputAnalogKeyState(EKeys::Gamepad_RightTriggerAxis);
	const FKey Buttons[15] = {
		EKeys::Gamepad_FaceButton_Bottom, EKeys::Gamepad_FaceButton_Right, EKeys::Gamepad_FaceButton_Left, EKeys::Gamepad_FaceButton_Top,
		EKeys::Gamepad_LeftShoulder, EKeys::Gamepad_RightShoulder, EKeys::Gamepad_Special_Left, EKeys::Gamepad_Special_Right,
		EKeys::Gamepad_LeftThumbstick, EKeys::Gamepad_RightThumbstick, EKeys::Gamepad_DPad_Up, EKeys::Gamepad_DPad_Down,
		EKeys::Gamepad_DPad_Left, EKeys::Gamepad_DPad_Right, EKeys::Invalid,
	};
	for (int32 i = 0; i < 15; ++i)
	{
		if (Buttons[i].IsValid() && PC->IsInputKeyDown(Buttons[i]))
		{
			Pad.buttons |= 1u << i;
		}
	}
	// Keyboard, using the same key codes as the browser/native builds.
	static const TPair<FKey, const TCHAR*> KeyMap[] = {
		{ EKeys::W, TEXT("KeyW") }, { EKeys::A, TEXT("KeyA") }, { EKeys::S, TEXT("KeyS") }, { EKeys::D, TEXT("KeyD") },
		{ EKeys::Up, TEXT("ArrowUp") }, { EKeys::Down, TEXT("ArrowDown") }, { EKeys::Left, TEXT("ArrowLeft") }, { EKeys::Right, TEXT("ArrowRight") },
		{ EKeys::SpaceBar, TEXT("Space") }, { EKeys::LeftShift, TEXT("ShiftLeft") }, { EKeys::LeftControl, TEXT("ControlLeft") },
		{ EKeys::H, TEXT("KeyH") }, { EKeys::Q, TEXT("KeyQ") }, { EKeys::E, TEXT("KeyE") }, { EKeys::X, TEXT("KeyX") }, { EKeys::L, TEXT("KeyL") },
		{ EKeys::K, TEXT("KeyK") }, { EKeys::J, TEXT("KeyJ") }, { EKeys::G, TEXT("KeyG") }, { EKeys::Y, TEXT("KeyY") }, { EKeys::B, TEXT("KeyB") },
		{ EKeys::C, TEXT("KeyC") }, { EKeys::T, TEXT("KeyT") }, { EKeys::R, TEXT("KeyR") }, { EKeys::F, TEXT("KeyF") }, { EKeys::Z, TEXT("KeyZ") },
		{ EKeys::N, TEXT("KeyN") }, { EKeys::I, TEXT("KeyI") }, { EKeys::P, TEXT("KeyP") }, { EKeys::M, TEXT("KeyM") }, { EKeys::V, TEXT("KeyV") },
		{ EKeys::BackSpace, TEXT("Backspace") }, { EKeys::One, TEXT("Digit1") }, { EKeys::Two, TEXT("Digit2") }, { EKeys::Three, TEXT("Digit3") },
	};
	for (const auto& K : KeyMap)
	{
		if (PC->IsInputKeyDown(K.Key))
		{
			Keys.Add(K.Value);
		}
	}
}

void AGridlockWorld::ApplyRumble()
{
	if (!bPlayerDrives || !FSlateApplication::IsInitialized())
	{
		return;
	}
	GlRumble R;
	gl_game_rumble(Game, &R);
	if (IInputInterface* Input = FSlateApplication::Get().GetInputInterface())
	{
		FForceFeedbackValues V;
		V.LeftLarge = R.low * RumbleStrength;
		V.RightLarge = R.low * RumbleStrength * 0.5f;
		V.LeftSmall = R.high * RumbleStrength * 0.5f;
		V.RightSmall = R.high * RumbleStrength;
		Input->SetForceFeedbackChannelValues(0, V);
	}
}

void AGridlockWorld::PumpEvents()
{
	GlEvent Events[64];
	const uint32 N = gl_game_events(Game, Events, 64);
	for (uint32 i = 0; i < N; ++i)
	{
		const GlEvent& E = Events[i];
		OnGridlockEvent.Broadcast((int32)E.kind, (int32)E.a, (int32)E.b, V3(E.pos), FString(UTF8_TO_TCHAR(E.text)));
	}
}

void AGridlockWorld::UpdateCamera(float Dt)
{
	if (!bPlayerDrives)
	{
		return;
	}
	CameraArm->SetWorldLocation(PlayerTransform.GetLocation() + FVector(0, 0, 120.0f));
	// Car forward is -Y in its frame; look along it with a little downward pitch.
	const FVector Fwd = PlayerTransform.TransformVectorNoScale(FVector(0, -1, 0));
	const FRotator Want(-10.0f, Fwd.Rotation().Yaw, 0.0f);
	CameraArm->SetWorldRotation(Want);
}

void AGridlockWorld::Tick(float DeltaSeconds)
{
	Super::Tick(DeltaSeconds);
	if (!Game)
	{
		return;
	}
	GlPad Pad;
	TArray<FString> Keys;
	ReadInput(Pad, Keys);
	// Key codes are ASCII; keep the buffers alive until the call returns.
	TArray<TArray<ANSICHAR>> KeyBufs;
	TArray<const char*> KeyPtrs;
	KeyBufs.SetNum(Keys.Num());
	for (int32 i = 0; i < Keys.Num(); ++i)
	{
		for (TCHAR Ch : Keys[i])
		{
			KeyBufs[i].Add((ANSICHAR)Ch);
		}
		KeyBufs[i].Add(0);
	}
	for (const TArray<ANSICHAR>& B : KeyBufs)
	{
		KeyPtrs.Add(B.GetData());
	}
	gl_game_update(Game, DeltaSeconds, &Pad, KeyPtrs.GetData(), KeyPtrs.Num());
	SyncCars();
	SyncSignals();
	SyncProps();
	SyncPeds();
	UpdateCamera(DeltaSeconds);
	ApplyRumble();
	PumpEvents();

	GlHud H;
	gl_game_hud(Game, &H);
	Hud.SpeedKmh = H.speed_kmh;
	Hud.Rpm = H.rpm;
	Hud.Redline = H.redline;
	Hud.LimitKmh = H.limit_kmh;
	Hud.Money = H.money;
	Hud.Heat = H.heat;
	Hud.EngineHealth = H.engine_health;
	Hud.Hour = H.hour;
	Hud.CruiseKmh = H.cruise_kmh;
	Hud.Points = (int32)H.points;
	Hud.MaxPoints = (int32)H.max_points;
	Hud.Gear = UTF8_TO_TCHAR(H.gear);
	Hud.Street = UTF8_TO_TCHAR(H.street);
	Hud.LeftIndicator = H.flags & 1;
	Hud.RightIndicator = (H.flags >> 1) & 1;
	Hud.Hazards = (H.flags >> 2) & 1;
	Hud.HighBeam = (H.flags >> 3) & 1;
	Hud.Seatbelt = (H.flags >> 4) & 1;
	Hud.AbsActive = (H.flags >> 6) & 1;
	Hud.TcsActive = (H.flags >> 7) & 1;
	Hud.EscActive = (H.flags >> 8) & 1;
}

bool AGridlockWorld::Action(const FString& ActionId)
{
	return Game && gl_game_action(Game, TCHAR_TO_UTF8(*ActionId)) != 0;
}

void AGridlockWorld::SetHour(float NewHour)
{
	if (Game)
	{
		gl_game_set_hour(Game, NewHour);
	}
}
