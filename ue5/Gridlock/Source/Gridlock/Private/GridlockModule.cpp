#include "GridlockModule.h"

#include "HAL/PlatformProcess.h"
#include "Interfaces/IPluginManager.h"
#include "Misc/Paths.h"

void FGridlockModule::StartupModule()
{
	// The DLL is delay-loaded; load it explicitly from the plugin's ThirdParty folder so the
	// editor finds it without copying it next to UnrealEditor.exe.
	const TSharedPtr<IPlugin> Plugin = IPluginManager::Get().FindPlugin(TEXT("Gridlock"));
	const FString Base = Plugin.IsValid() ? Plugin->GetBaseDir() : FPaths::Combine(FPaths::ProjectPluginsDir(), TEXT("Gridlock"));
	const FString Dir = FPaths::Combine(Base, TEXT("Source/ThirdParty/GridlockFFI/lib/Win64"));
	const FString Dll = FPaths::Combine(Dir, TEXT("gridlock_ffi.dll"));
	FPlatformProcess::PushDllDirectory(*Dir);
	DllHandle = FPlatformProcess::GetDllHandle(*Dll);
	FPlatformProcess::PopDllDirectory(*Dir);
	if (!DllHandle)
	{
		UE_LOG(LogTemp, Error, TEXT("Gridlock: could not load %s. Build it with 'cargo build --release -p gridlock-ffi' and copy gridlock_ffi.dll + gridlock_ffi.dll.lib there."), *Dll);
	}
}

void FGridlockModule::ShutdownModule()
{
	if (DllHandle)
	{
		FPlatformProcess::FreeDllHandle(DllHandle);
		DllHandle = nullptr;
	}
}

IMPLEMENT_MODULE(FGridlockModule, Gridlock)
