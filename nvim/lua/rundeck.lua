local M = {}

local defaults = {
	command = "rundeck",
	keymaps = {
		open = "<leader>rd",
		add = "<leader>ra",
		create = "<leader>rc",
		config = "<leader>re",
	},
}

local opts = vim.deepcopy(defaults)

local function get_cwd()
	return vim.loop.cwd() or vim.fn.getcwd()
end

local function shell_escape(value)
	return vim.fn.shellescape(value)
end

local function project_name_from_cwd()
	return vim.fn.fnamemodify(get_cwd(), ":t")
end

local function run_in_terminal(cmd)
	vim.cmd("botright split")
	vim.cmd("resize 12")
	vim.cmd("terminal " .. cmd)
	vim.cmd("startinsert")
end

function M.open()
	local cmd = "tmux new-session -A -s rundeck-dashboard " .. opts.command
	vim.cmd("silent !" .. cmd)
	vim.cmd("redraw!")
end

function M.add_current_project()
	local cwd = get_cwd()
	local name = project_name_from_cwd()

	local cmd = table.concat({
		opts.command,
		"add",
		shell_escape(cwd),
		"--name",
		shell_escape(name),
	}, " ")

	run_in_terminal(cmd)
end

function M.create_project()
	vim.ui.input({ prompt = "Project name: " }, function(name)
		if not name or name == "" then
			return
		end

		vim.notify("Open RunDeck and press c to create: " .. name, vim.log.levels.INFO)
		M.open()
	end)
end

function M.open_config()
	local config_path = vim.fn.expand("~/.config/rundeck/config.toml")
	vim.cmd("edit " .. vim.fn.fnameescape(config_path))
end

function M.setup(user_opts)
	opts = vim.tbl_deep_extend("force", defaults, user_opts or {})

	vim.api.nvim_create_user_command("Rundeck", M.open, {})
	vim.api.nvim_create_user_command("RundeckAdd", M.add_current_project, {})
	vim.api.nvim_create_user_command("RundeckCreate", M.create_project, {})
	vim.api.nvim_create_user_command("RundeckConfig", M.open_config, {})

	vim.keymap.set("n", opts.keymaps.open, M.open, { desc = "RunDeck dashboard" })
	vim.keymap.set("n", opts.keymaps.add, M.add_current_project, { desc = "RunDeck add current project" })
	vim.keymap.set("n", opts.keymaps.create, M.create_project, { desc = "RunDeck create project" })
	vim.keymap.set("n", opts.keymaps.config, M.open_config, { desc = "RunDeck config" })
end

return M
