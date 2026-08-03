#pragma once

#include "IManagerDataProvider.h"

#include <memory>

namespace Pastral::Manager::Presentation
{
    std::shared_ptr<IManagerDataProvider> CreateManagerDataProvider();
}
