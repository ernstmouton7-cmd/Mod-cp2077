#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "gridlock.h"
#include "GridlockWorld.generated.h"

class UProceduralMeshComponent;
class UStaticMeshComponent;
class USpotLightComponent;
class UPointLightComponent;
class USpringArmComponent;
class UCameraComponent;
class UMaterialInterface;
class UMaterialInstanceDynamic;
class UStaticMesh;

UENUM(BlueprintType)
enum class EGridlockVehicleClass : uint8
{
	Compact, Sedan, Sports, Muscle, Suv, Van, Truck, Bus, Taxi, Police, Ambulance
};

/** Player HUD values, refreshed every tick (bind these in UMG). */
USTRUCT(BlueprintType)
struct FGridlockHud
{
	GENERATED_BODY()
	UPROPERTY(BlueprintReadOnly) float SpeedKmh = 0;
	UPROPERTY(BlueprintReadOnly) float Rpm = 0;
	UPROPERTY(BlueprintReadOnly) float Redline = 0;
	UPROPERTY(BlueprintReadOnly) float LimitKmh = 0;
	UPROPERTY(BlueprintReadOnly) float Money = 0;
	UPROPERTY(BlueprintReadOnly) float Heat = 0;
	UPROPERTY(BlueprintReadOnly) float EngineHealth = 1;
	UPROPERTY(BlueprintReadOnly) float Hour = 0;
	UPROPERTY(BlueprintReadOnly) float CruiseKmh = -1;
	UPROPERTY(BlueprintReadOnly) int32 Points = 0;
	UPROPERTY(BlueprintReadOnly) int32 MaxPoints = 12;
	UPROPERTY(BlueprintReadOnly) FString Gear;
	UPROPERTY(BlueprintReadOnly) FString Street;
	UPROPERTY(BlueprintReadOnly) bool LeftIndicator = false;
	UPROPERTY(BlueprintReadOnly) bool RightIndicator = false;
	UPROPERTY(BlueprintReadOnly) bool Hazards = false;
	UPROPERTY(BlueprintReadOnly) bool HighBeam = false;
	UPROPERTY(BlueprintReadOnly) bool Seatbelt = false;
	UPROPERTY(BlueprintReadOnly) bool AbsActive = false;
	UPROPERTY(BlueprintReadOnly) bool TcsActive = false;
	UPROPERTY(BlueprintReadOnly) bool EscActive = false;
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_FiveParams(FGridlockEventSignature, int32, Kind, int32, A, int32, B, FVector, Location, const FString&, Text);

/** One rendered car: body and part sections, four wheels, lamps. */
struct FGridlockCarVisual
{
	TObjectPtr<UProceduralMeshComponent> Body = nullptr;
	TArray<TObjectPtr<UProceduralMeshComponent>> Parts;
	TArray<TObjectPtr<UProceduralMeshComponent>> Wheels;
	TObjectPtr<UMaterialInstanceDynamic> Paint = nullptr;
	TObjectPtr<UMaterialInstanceDynamic> HeadLamps = nullptr;
	TObjectPtr<UMaterialInstanceDynamic> TailLamps = nullptr;
	uint32 ClassIndex = 0;
	uint64 BodyRevision = 0;
	bool bSeen = false;
};

/**
 * Hosts a whole Gridlock city: traffic with AI drivers and road rage, signals, pedestrians,
 * law enforcement, full vehicle physics with crash deformation, breakable street furniture.
 * Place one in an empty level, assign materials, press Play. Drive with an Xbox pad or WASD.
 */
UCLASS(Blueprintable)
class GRIDLOCK_API AGridlockWorld : public AActor
{
	GENERATED_BODY()

public:
	AGridlockWorld();

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gridlock") int64 Seed = 2077;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gridlock") int32 TrafficCount = 70;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gridlock") int32 PoliceCount = 2;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gridlock", meta = (ClampMin = 0, ClampMax = 24)) float StartHour = 17.5f;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gridlock") EGridlockVehicleClass PlayerVehicle = EGridlockVehicleClass::Sedan;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gridlock") bool bSpawnProps = true;
	/** Possess the camera and read the first local player's gamepad/keyboard. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gridlock") bool bPlayerDrives = true;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gridlock") float RumbleStrength = 1.0f;
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gridlock") int32 MaxStreetLights = 96;
	/** Flip all generated triangles. Tick this if cars and buildings look inside-out. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "Gridlock") bool bReverseWinding = false;

	/** Car paint; needs a vector parameter "Color". */
	UPROPERTY(EditAnywhere, Category = "Gridlock|Materials") TObjectPtr<UMaterialInterface> PaintMaterial;
	UPROPERTY(EditAnywhere, Category = "Gridlock|Materials") TObjectPtr<UMaterialInterface> TrimMaterial;
	UPROPERTY(EditAnywhere, Category = "Gridlock|Materials") TObjectPtr<UMaterialInterface> GlassMaterial;
	/** Lamps; need a scalar parameter "Emissive" and a vector parameter "Color". */
	UPROPERTY(EditAnywhere, Category = "Gridlock|Materials") TObjectPtr<UMaterialInterface> LampMaterial;
	UPROPERTY(EditAnywhere, Category = "Gridlock|Materials") TObjectPtr<UMaterialInterface> TyreMaterial;
	UPROPERTY(EditAnywhere, Category = "Gridlock|Materials") TObjectPtr<UMaterialInterface> RimMaterial;
	UPROPERTY(EditAnywhere, Category = "Gridlock|Materials") TObjectPtr<UMaterialInterface> RoadMaterial;
	UPROPERTY(EditAnywhere, Category = "Gridlock|Materials") TObjectPtr<UMaterialInterface> SidewalkMaterial;
	UPROPERTY(EditAnywhere, Category = "Gridlock|Materials") TObjectPtr<UMaterialInterface> GrassMaterial;
	/** Buildings; optional vector parameter "Color". UVs are in metres. */
	UPROPERTY(EditAnywhere, Category = "Gridlock|Materials") TObjectPtr<UMaterialInterface> BuildingMaterial;
	UPROPERTY(EditAnywhere, Category = "Gridlock|Materials") TObjectPtr<UMaterialInterface> MarkingWhiteMaterial;
	UPROPERTY(EditAnywhere, Category = "Gridlock|Materials") TObjectPtr<UMaterialInterface> MarkingYellowMaterial;

	UPROPERTY(BlueprintReadOnly, Category = "Gridlock") FGridlockHud Hud;
	UPROPERTY(BlueprintAssignable, Category = "Gridlock") FGridlockEventSignature OnGridlockEvent;

	/** Fire an action by id, e.g. "indicator_left", "horn", "apologize" (UI buttons). */
	UFUNCTION(BlueprintCallable, Category = "Gridlock") bool Action(const FString& ActionId);
	UFUNCTION(BlueprintCallable, Category = "Gridlock") void SetHour(float NewHour);
	UFUNCTION(BlueprintPure, Category = "Gridlock") FTransform GetPlayerTransform() const { return PlayerTransform; }

	virtual void Tick(float DeltaSeconds) override;

protected:
	virtual void BeginPlay() override;
	virtual void EndPlay(const EEndPlayReason::Type Reason) override;

private:
	void BuildCity();
	void SyncCars();
	void SyncSignals();
	void SyncProps();
	void SyncPeds();
	void UpdateCamera(float Dt);
	void ReadInput(GlPad& Pad, TArray<FString>& Keys) const;
	void ApplyRumble();
	void PumpEvents();
	FGridlockCarVisual& CarVisual(uint32 Id, uint32 ClassIndex);
	void AddSection(UProceduralMeshComponent* Mesh, int32 Index, const TArray<FVector>& V, TArray<int32> T, const TArray<FVector>& N, const TArray<FVector2D>& UV, const TArray<FColor>& Col, bool bCollision, UMaterialInterface* Material);
	void FillSection(UProceduralMeshComponent* Mesh, int32 Section, const GlMesh& M, UMaterialInterface* Material, bool bUpdateOnly = false);
	UMaterialInterface* PaintSlot(uint32 Paint, FGridlockCarVisual& V) const;

	GlGame* Game = nullptr;

	UPROPERTY() TObjectPtr<USceneComponent> Root;
	UPROPERTY() TObjectPtr<UProceduralMeshComponent> CityMesh;
	UPROPERTY() TObjectPtr<USpringArmComponent> CameraArm;
	UPROPERTY() TObjectPtr<UCameraComponent> Camera;
	UPROPERTY() TObjectPtr<USpotLightComponent> HeadlightL;
	UPROPERTY() TObjectPtr<USpotLightComponent> HeadlightR;
	UPROPERTY() TArray<TObjectPtr<UPointLightComponent>> StreetLights;
	UPROPERTY() TArray<TObjectPtr<UStaticMeshComponent>> PropMeshes;
	UPROPERTY() TArray<TObjectPtr<UStaticMeshComponent>> SignalLamps;
	UPROPERTY() TArray<TObjectPtr<UMaterialInstanceDynamic>> SignalMaterials;
	UPROPERTY() TArray<TObjectPtr<UStaticMeshComponent>> PedMeshes;
	UPROPERTY() TArray<TObjectPtr<UObject>> KeepAlive;

	TMap<uint32, FGridlockCarVisual> Cars;
	TArray<GlCarView> CarBuffer;
	TArray<GlSignalView> SignalBuffer;
	TArray<GlPropView> PropBuffer;
	TArray<GlPedView> PedBuffer;
	FTransform PlayerTransform;
	UPROPERTY() TObjectPtr<UStaticMesh> Cube;
	UPROPERTY() TObjectPtr<UStaticMesh> Cylinder;
	UPROPERTY() TObjectPtr<UStaticMesh> Sphere;
};
