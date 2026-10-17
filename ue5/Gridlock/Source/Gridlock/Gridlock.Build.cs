using System.IO;
using UnrealBuildTool;

public class Gridlock : ModuleRules
{
	public Gridlock(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;
		PublicDependencyModuleNames.AddRange(new string[] { "Core", "CoreUObject", "Engine", "InputCore", "ProceduralMeshComponent" });
		PrivateDependencyModuleNames.AddRange(new string[] { "Projects", "ApplicationCore", "Slate", "SlateCore" });

		string ThirdParty = Path.Combine(ModuleDirectory, "..", "ThirdParty", "GridlockFFI");
		PublicIncludePaths.Add(Path.Combine(ThirdParty, "include"));

		if (Target.Platform == UnrealTargetPlatform.Win64)
		{
			string LibDir = Path.Combine(ThirdParty, "lib", "Win64");
			// Import library for gridlock_ffi.dll (built with: cargo build --release -p gridlock-ffi).
			PublicAdditionalLibraries.Add(Path.Combine(LibDir, "gridlock_ffi.dll.lib"));
			PublicDelayLoadDLLs.Add("gridlock_ffi.dll");
			RuntimeDependencies.Add("$(BinaryOutputDir)/gridlock_ffi.dll", Path.Combine(LibDir, "gridlock_ffi.dll"));
		}
	}
}
