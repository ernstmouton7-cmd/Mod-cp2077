#pragma once

#include "Modules/ModuleManager.h"

class FGridlockModule : public IModuleInterface
{
public:
	virtual void StartupModule() override;
	virtual void ShutdownModule() override;

private:
	void* DllHandle = nullptr;
};
