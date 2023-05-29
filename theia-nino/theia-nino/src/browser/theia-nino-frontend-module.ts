/**
 * Generated using theia-extension-generator
 */
import { TheiaNinoCommandContribution, TheiaNinoMenuContribution } from './theia-nino-contribution';
import { CommandContribution, MenuContribution } from '@theia/core/lib/common';
import { ContainerModule } from '@theia/core/shared/inversify';

export default new ContainerModule(bind => {
    // add your contribution bindings here
    bind(CommandContribution).to(TheiaNinoCommandContribution);
    bind(MenuContribution).to(TheiaNinoMenuContribution);
});
