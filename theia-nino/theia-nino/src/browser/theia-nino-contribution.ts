import { injectable, inject } from '@theia/core/shared/inversify';
import { Command, CommandContribution, CommandRegistry, MenuContribution, MenuModelRegistry, MessageService } from '@theia/core/lib/common';
import { CommonMenus } from '@theia/core/lib/browser';

export const TheiaNinoCommand: Command = {
    id: 'TheiaNino.command',
    label: 'Say Hello'
};

@injectable()
export class TheiaNinoCommandContribution implements CommandContribution {

    constructor(
        @inject(MessageService) private readonly messageService: MessageService,
    ) { }

    registerCommands(registry: CommandRegistry): void {
        registry.registerCommand(TheiaNinoCommand, {
            execute: () => this.messageService.info('mesage saying "Hello"')
        });
    }
}

@injectable()
export class TheiaNinoMenuContribution implements MenuContribution {

    registerMenus(menus: MenuModelRegistry): void {
        menus.registerMenuAction(CommonMenus.EDIT_FIND, {
            commandId: TheiaNinoCommand.id,
            label: TheiaNinoCommand.label
        });
    }
}
